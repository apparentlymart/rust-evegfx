use super::command_word::CommandWord;
use super::strfmt;
use crate::commands::options;
use crate::commands::waiter::{PollingWaiter, Waiter, WaiterError};
use crate::interface::Interface;
use crate::low_level::LowLevel;
use crate::memory::{Ptr, Slice};
use crate::models::Model;
use crate::registers::Register;

/// The result type for coprocessor operations, where the error type is always
/// [`Error`](Error).
pub type Result<T, M, I, W> =
    core::result::Result<T, Error<<I as Interface>::Error, <W as Waiter<M, I>>::Error>>;

/// An interface to the command ring buffer for the EVE chip's coprocessor
/// component.
///
/// This object encapsulates the handling of the ring buffer and provides an
/// API for appending
pub struct Coprocessor<M: Model, I: Interface, W: Waiter<M, I>> {
    ll: LowLevel<M, I>,
    wait: W,

    // `known_space` tracks the amount of available buffer space (in bytes) that
    // we most recently knew about. The coprocessor asynchronously consumes
    // command words from the ring buffer, so there might actually be _more_
    // space than reported here, but there should always be at least this much
    // space because we keep decreasing this as we write more data into the
    // buffer.
    //
    // Once this value gets too low to append any more commands, we'll use
    // the waiter to wait for more space and then update `known_space` with
    // the new free space determined by the waiter.
    known_space: u16,
}

/// The methods which submit new commands into the coprocessor ringbuffer.
///
/// These will block using the waiter if they run out of coprocessor buffer
/// space, but they will not wait if there's enough buffer space available to
/// write into.
///
/// Although these commands do block for there being sufficient buffer space
/// to write them, they _don't_ wait for the coprocessor to actually execute
/// the instructions, because the goal is for the coprocessor to primarily
/// run concurrently with code on the host processor. However, there are
/// some methods which _do_ block for the completion of certain operations,
/// which all have the name prefix `block_` to indicate that.
impl<M: Model, I: Interface, W: Waiter<M, I>> Coprocessor<M, I, W> {
    /// A convenience function for enclosing a series of coprocessor commands
    /// in `start_display_list` and `display_list_swap` commands.
    ///
    /// The given closure can in principle call all of the same methods as
    /// directly on the coprocessor object, but it's best to avoid any action
    /// that interacts with anything outside of the coprocessor. It _definitely_
    /// doesn't make sense to recursively call into `new_display_list` again.
    pub fn new_display_list<F>(&mut self, f: F) -> Result<(), M, I, W>
    where
        F: FnOnce(&mut Self) -> Result<(), M, I, W>,
    {
        self.start_display_list()?;
        f(self)?;
        self.display_list_swap()
    }

    /// Wait for at least the given delay and then have the coprocessor trigger
    /// the EVE interrupt `CMDFLAG`.
    ///
    /// In a system that's able to respond to interrupts from the EVE chip,
    /// you can use this to get a proactive notification when the coprocessor
    /// reaches a particular point in the command stream, such as if you are
    /// waiting for a particular command to be run before taking some other
    /// action that isn't controlled by the EVE coprocessor but must still be
    /// synchronized with it.
    ///
    /// Use a duration of zero (i.e. `Duration::from_millis(0)` or similar)
    /// to have the coprocessor fire the interrupt immediately on decoding
    /// this command, without any extra delay.
    ///
    /// If you use a delay of more than 4,294,967 seconds then the delay
    /// sent to the EVE chip will saturate at that amount of seconds, due
    /// to the physical constraint on number of bits available to express
    /// the precision in the protocol.
    pub fn trigger_cmdflag_interrupt(
        &mut self,
        delay: core::time::Duration,
    ) -> Result<(), M, I, W> {
        self.write_stream(8, |cp| {
            let delay = delay.as_millis();
            let delay = if delay > (core::u32::MAX as u128) {
                core::u32::MAX
            } else {
                delay as u32
            };
            cp.write_to_buffer(0xFFFFFF02 as u32)?;
            cp.write_to_buffer(delay)
        })
    }

    /// Resets the coprocessor's state to the boot-time defaults before
    /// continuing with later commands. For example, this discards the
    /// currently-selected widget colors and reverts to the default color
    /// scheme.
    pub fn cold_start(&mut self) -> Result<(), M, I, W> {
        self.write_stream(4, |cp| cp.write_to_buffer(0xFFFFFF32 as u32))
    }

    /// Reads a slice of display list command bytes from a location in the main
    /// memory space.
    ///
    /// You can use this, for example, to keep various pre-built display list
    /// sequences prepared in the main EVE RAM and then efficiently retrieve
    /// them into display list memory without having to re-transmit them
    /// from host CPU to EVE. However, these commands do ultimately still
    /// take real space in the display list memory after the command completes.
    ///
    /// The bounds of the given slice must both be addresses that are a
    /// multiple of four, to respect the required memory alignment.
    pub fn append_display_list_from_main_mem<S: Into<Slice<M::MainMem>>>(
        &mut self,
        slice: S,
    ) -> Result<(), M, I, W> {
        let slice: Slice<M::MainMem> = slice.into();
        let start = slice.start().to_raw();
        let count = slice.len();
        self.write_stream(12, |cp| {
            cp.write_to_buffer(0xFFFFFF1E as u32)?;
            cp.write_to_buffer(start)?;
            cp.write_to_buffer(count)
        })
    }

    pub fn show_testcard(&mut self) -> Result<(), M, I, W> {
        self.write_stream(4, |cp| cp.write_to_buffer(0xFFFFFF61 as u32))
    }

    pub fn show_manufacturer_logo(&mut self) -> Result<(), M, I, W> {
        self.write_stream(4, |cp| cp.write_to_buffer(0xFFFFFF31 as u32))
    }

    pub fn start_spinner(&mut self) -> Result<(), M, I, W> {
        // TODO: Make the spinner customizable.
        self.write_stream(20, |cp| {
            cp.write_to_buffer(0xFFFFFF16 as u32)?;
            cp.write_to_buffer(1000)?;
            cp.write_to_buffer(1000)?;
            cp.write_to_buffer(0)?;
            cp.write_to_buffer(0)
        })
    }

    /// Sends just the coprocessor command to start a new display list, which
    /// waits for the display list memory to become writable before executing
    /// any subsequent commands and resets the pointer for new display list
    /// commands back to the top of display list memory.
    ///
    /// It's generally more convenient to use
    /// [`new_display_list`](Coprocessor::new_display_list), which
    /// handles both starting the display list and swapping it to be visible
    /// all in a single method call.
    pub fn start_display_list(&mut self) -> Result<(), M, I, W> {
        self.write_stream(4, |cp| cp.write_to_buffer(0xFFFFFF00 as u32))
    }

    /// Sends just the coprocessor command to swap in the newly-populated
    /// display list commands.
    ///
    /// It's generally more convenient to use
    /// [`new_display_list`](Coprocessor::new_display_list), which
    /// handles both starting the display list and swapping it to be visible
    /// all in a single method call.
    pub fn display_list_swap(&mut self) -> Result<(), M, I, W> {
        self.write_stream(4, |cp| cp.write_to_buffer(0xFFFFFF01 as u32))
    }

    pub fn draw_button<Rect: Into<crate::graphics::WidgetRect>>(
        &mut self,
        rect: Rect,
        msg: strfmt::Message<M::MainMem>,
        font: options::FontRef,
        options: options::Button,
    ) -> Result<(), M, I, W> {
        let rect: crate::graphics::WidgetRect = rect.into();
        self.write_stream(28, |cp| {
            cp.write_to_buffer(0xFFFFFF0D as u32)?;
            cp.write_to_buffer((rect.x, rect.y))?;
            cp.write_to_buffer((rect.w, rect.h))?;
            let font_raw = font.to_raw() as u16;
            let opts_raw = maybe_opt_format(options.to_raw(), &msg) as u16;
            cp.write_to_buffer((font_raw, opts_raw))
        })?;
        self.write_fmt_message(&msg)
    }

    pub fn draw_text<Pos: Into<crate::graphics::WidgetPos>>(
        &mut self,
        pos: Pos,
        msg: strfmt::Message<M::MainMem>,
        font: options::FontRef,
        options: options::Text,
    ) -> Result<(), M, I, W> {
        let pos: crate::graphics::WidgetPos = pos.into();
        self.write_stream(28, |cp| {
            cp.write_to_buffer(0xFFFFFF0C as u32)?;
            cp.write_to_buffer((pos.x, pos.y))?;
            let font_raw = font.to_raw() as u16;
            let opts_raw = maybe_opt_format(options.to_raw(), &msg) as u16;
            cp.write_to_buffer((font_raw, opts_raw))
        })?;
        self.write_fmt_message(&msg)
    }

    pub fn append_display_list(&mut self, cmd: crate::display_list::DLCmd) -> Result<(), M, I, W> {
        self.write_stream(4, |cp| cp.write_to_buffer(cmd.as_raw()))
    }

    pub fn append_raw_word(&mut self, word: u32) -> Result<(), M, I, W> {
        self.write_stream(4, |cp| cp.write_to_buffer(word))
    }

    pub fn wait_microseconds(&mut self, delay: u32) -> Result<(), M, I, W> {
        self.write_stream(8, |cp| {
            cp.write_to_buffer(0xFFFFFF65 as u32)?;
            cp.write_to_buffer(delay)
        })
    }

    pub fn wait_video_scanout(&mut self) -> Result<(), M, I, W> {
        self.write_stream(4, |cp| cp.write_to_buffer(0xFFFFFF42 as u32))
    }
}

/// The methods which block until the coprocessor has "caught up" with
/// particular events.
///
/// These make use of the associated "waiter" to block until specific
/// coprocessor commands have completed, and so applications making heavy
/// use of these may wish to consider supplying a tailored waiter
/// implementation that can avoid busy-waiting.
impl<M: Model, I: Interface, W: Waiter<M, I>> Coprocessor<M, I, W> {
    #[inline]
    pub fn space_when_empty() -> u16 {
        // Perhaps this will vary in future models, but it's always been
        // consistent so far so we'll make this more complex only when needed.
        4092
    }

    /// Blocks until the coprocessor buffer is empty, signalling that the
    /// coprocessor has completed all of the commands issued so far.
    pub fn block_until_idle(&mut self) -> Result<(), M, I, W> {
        self.ensure_space(Self::space_when_empty())
    }

    /// Blocks until EVE has finished scanning out the current frame. Callers
    /// can use this as part of a main loop which takes actions synchronized
    /// with the video framerate.
    ///
    /// This is a blocking version of `wait_video_scanout`.
    pub fn block_until_video_scanout(&mut self) -> Result<(), M, I, W> {
        self.wait_video_scanout()?;
        self.block_until_idle()
    }

    /// Blocks until the coprocessor has compelted all of the commands issued
    /// so far and then returns the value of the given system register.
    ///
    /// You can use this in situations where earlier coprocessor commands may
    /// have modified the register value, in order to capture that result
    /// at the correct time.
    pub fn block_read_register(&mut self, reg: crate::registers::Register) -> Result<u32, M, I, W> {
        let ptr = M::reg_ptr(reg);

        self.write_stream(12, |cp| {
            cp.write_to_buffer(0xFFFFFF19 as u32)?;
            cp.write_to_buffer(ptr.to_raw())?;
            cp.write_to_buffer(0xf0f0f0f0 as u32) // space for the result to be written
        })?;

        // This command has space allocated in it where the coprocessor will
        // write the result, so we'll need to grab the current write pointer
        // after we write this out and then wait for the value to appear.
        let ptr_reg = crate::registers::Register::CMD_WRITE;
        let stopped = self.stop_stream()?;
        let write_addr = {
            let ll = self.borrow_low_level(&stopped);
            Self::interface_result(ll.rd32(M::reg_ptr(ptr_reg)))?
        };

        // wait for the coprocessor to catch up
        self.ensure_space_stopped(&stopped, Self::space_when_empty())?;

        let result = {
            let result_ptr: Ptr<M::DisplayListMem> = Ptr::new(write_addr) - 4;
            let ll = self.borrow_low_level(&stopped);
            Self::interface_result(ll.rd32(result_ptr))
        };

        self.start_stream(stopped)?;
        result
    }
}

impl<M: Model, I: Interface, W: Waiter<M, I>> Coprocessor<M, I, W> {
    /// Consumes the given interface and waiter and returns an interface to
    /// the coprocessor via the given interface.
    ///
    /// This function consumes the interface because it will be constantly
    /// writing into the command ring buffer of the associated EVE chip and
    /// so it isn't safe to do any other concurrent access. You can get
    /// the underlying interface back again if you need it using some of
    /// the methods of `Coprocessor`.
    pub fn new(ei: I, wait: W) -> Result<Self, M, I, W> {
        let mut ll = crate::low_level::LowLevel::new(ei);

        // We'll pulse the reset signal for the coprocessor just to make sure
        // we're finding it in a known good state.
        Self::interface_result(ll.wr8(ll.reg_ptr(Register::CPURESET), 0b001))?;
        Self::interface_result(ll.wr8(ll.reg_ptr(Register::CPURESET), 0b000))?;

        let mut ret = Self {
            ll: ll,
            wait: wait,
            known_space: 0,
        };

        // We use a "stopped stream" marker to help ensure correct discipline
        // around which actions must be taken with the stream active or
        // stopped, but we need to get that process started here by minting
        // our first "stopped stream" token to represent that the stream
        // isn't running until we call start_stream for the first time below.
        let stopped = StoppedStream;

        // Copy the current values for free space and next offset into our
        // local fields so we will start off in sync with the remote chip.
        ret.synchronize(&stopped)?;

        // We keep our write transaction open any time we're not
        // resynchronizing or waiting, since that allows us to burst writes
        // into the command FIFO.
        ret.start_stream(stopped)?;

        Ok(ret)
    }

    /// Consumes the current coprocessor object and then returns a new one
    /// that's the same except that it has a new waiter, which is possibly
    /// derived from the previous one.
    ///
    /// The main goal here is to allow replacing the waiter with a wrapper
    /// implementation that does additional logging or tracking of waiting,
    /// if needed for debugging or development, without needing to first
    /// determine what kind of waiter the object previously had.
    pub fn with_new_waiter<W2, F>(self, f: F) -> Coprocessor<M, I, W2>
    where
        W2: Waiter<M, I>,
        F: FnOnce(W) -> W2,
    {
        let ll = self.ll;
        let old_wait = self.wait;
        let old_known_space = self.known_space;

        let new_wait = f(old_wait);

        Coprocessor {
            ll: ll,
            wait: new_wait,
            known_space: old_known_space,
        }
    }

    /// `take_interface` consumes the coprocessor object and returns its
    /// underlying `Interface`.
    ///
    /// To make temporary use of the underlying interface, without also
    /// discarding the coprocessor object, use `with_interface` instead.
    pub fn take_interface(mut self) -> Result<I, M, I, W> {
        self.stop_stream()?;
        return Ok(self.ll.take_interface());
    }

    /// `with_interface` runs your given closure with access to the
    /// coprocessor object's underlying `Interface`, temporarily pausing
    /// local coprocessor management so the closure can make use of other
    /// chip functionality.
    pub fn with_interface<R, F: FnOnce(&mut I) -> Result<R, M, I, W>>(
        &mut self,
        f: F,
    ) -> Result<R, M, I, W> {
        let stopped = self.stop_stream()?;
        let result = {
            let ei = self.ll.borrow_interface();
            f(ei)
        };
        // The caller could've messed with the registers we depend on, so
        // we'll resynchronize them before we restart our write stream.
        self.synchronize(&stopped)?;
        self.start_stream(stopped)?;
        result
    }

    // Update our internal records to match the state of the remote chip.
    fn synchronize(&mut self, _stopped: &StoppedStream) -> Result<(), M, I, W> {
        let known_space =
            Self::interface_result(self.ll.rd16(self.ll.reg_ptr(Register::CMDB_SPACE)))?;
        self.known_space = known_space;
        Ok(())
    }

    fn borrow_interface<'a>(&'a mut self, stopped: &StoppedStream) -> &'a mut I {
        let ll = self.borrow_low_level(stopped);
        ll.borrow_interface()
    }

    fn borrow_low_level<'a>(&'a mut self, _stopped: &StoppedStream) -> &'a mut LowLevel<M, I> {
        &mut self.ll
    }

    fn borrow_low_level_and_waiter<'a>(
        &'a mut self,
        _stopped: &StoppedStream,
    ) -> (&'a mut LowLevel<M, I>, &'a mut W) {
        (&mut self.ll, &mut self.wait)
    }

    // `start_stream` consumes the StoppedStream token because by the time it
    // returns the stream isn't stopped anymore.
    fn start_stream(&mut self, stopped: StoppedStream) -> Result<(), M, I, W> {
        // We now begin a write transaction at the next offset, so subsequent
        // command writes can just go directly into that active transaction.
        // This relies on the fact that EVE has a special behavior where
        // writes into RAM_CMD wrap around only inside the command space, not
        // in the whole memory space, and so we can just keep writing and let
        // the chip worry about the wraparound for us.
        let ei = self.borrow_interface(&stopped);
        Self::interface_result(ei.begin_write(M::reg_ptr(Register::CMDB_WRITE).to_raw()))
    }

    // `stop_stream` produces a StoppedStream token to represent that it has
    // stopped the stream and thus the caller can safely perform operations
    // that expect the stream to be stopped.
    fn stop_stream(&mut self) -> Result<StoppedStream, M, I, W> {
        // This just closes the long-lived write transaction we started in
        // start_stream.
        let ei = self.ll.borrow_interface();
        Self::interface_result(ei.end_write())?;
        Ok(StoppedStream)
    }

    fn write_stream<F: FnOnce(&mut Self) -> Result<(), M, I, W>>(
        &mut self,
        len: u16,
        f: F,
    ) -> Result<(), M, I, W> {
        self.ensure_space(len)?;

        // We just assume that our stream will always be active here and
        // so we can just burst writes into it. It's the responsibility of
        // any function that takes actions other than writing into the FIFO
        // to temporarily stop and then restart the stream.Register
        f(self)?;

        Ok(())
    }

    // Block using our waiter until there's at least `need` bytes of free space
    // in the ring buffer.
    fn ensure_space(&mut self, need: u16) -> Result<(), M, I, W> {
        if self.known_space >= need {
            // Fast path: our local tracking knows there's enough space. In
            // this case we can avoid stopping our burst-writing stream, which
            // allows for better write throughput.
            return Ok(());
        }

        // Otherwise we need to use our waiter to block until there's
        // enough space, and then update our record of known_space in the
        // hope of using the fast path next time. We do need to pause the
        // burst stream in this case, because the waiter will need to make
        // other calls against the EVE chip.
        let stopped = self.stop_stream()?;
        self.ensure_space_stopped(&stopped, need)?;
        self.start_stream(stopped)?;
        Ok(())
    }

    // A version of `ensure_space` that assumes the stream is already stopped
    // and will remain stopped after it returns.
    fn ensure_space_stopped(&mut self, stopped: &StoppedStream, need: u16) -> Result<(), M, I, W> {
        if self.known_space >= need {
            return Ok(());
        }

        let (ll, wait) = self.borrow_low_level_and_waiter(&stopped);
        match wait.wait_for_space(ll, need) {
            Ok(known_space) => {
                self.known_space = known_space;
            }
            Err(err) => {
                // We don't know how much space we have, so we'll set it
                // to zero to force calling the waiter again next time.
                self.known_space = 0;

                return Err(match err {
                    WaiterError::Comm(err) => Error::Waiter(err),
                    WaiterError::Fault => Error::Fault,
                });
            }
        }
        Ok(())
    }

    // Write directly to the output stream. This function doesn't check whether
    // there's sufficient space in the buffer, so the caller should call
    // ensure_space first to wait until there's enough space for the full
    // message it intends to write.
    fn write_to_buffer<V: Into<CommandWord>>(&mut self, v: V) -> Result<(), M, I, W> {
        let v: CommandWord = v.into();
        let v = v.to_raw();
        let data: [u8; 4] = [v as u8, (v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8];
        let ei = self.ll.borrow_interface();
        let result = Self::interface_result(ei.continue_write(&data));

        // We assume we consumed some buffer space even if there was an error,
        // because we can't actually tell if we did or not but reducing our
        // known minimum just means that we'll resync this value from the
        // real device sooner, recovering the correct value.
        if self.known_space >= 4 {
            self.known_space -= 4;
        }
        result
    }

    // Write a series of bytes into the output stream in chunks, with null
    // padding at the end to ensure that the message ends on a four-byte
    // word boundary.
    fn write_bytes_chunked(&mut self, v: &[u8]) -> Result<(), M, I, W> {
        for word in super::command_word::command_words_for_bytes(v) {
            self.ensure_space(4)?;
            self.write_to_buffer(word.to_raw())?;
        }
        Ok(())
    }

    fn write_fmt_message<R: crate::memory::MainMem>(
        &mut self,
        msg: &strfmt::Message<'_, '_, R>,
    ) -> Result<(), M, I, W> {
        use strfmt::Argument::*;
        self.write_bytes_chunked(msg.fmt)?;
        if let Some(args) = msg.args {
            let arg_space = (args.len() * 4) as u16;
            self.ensure_space(arg_space)?;
            for arg in args {
                let raw: u32 = match *arg {
                    Int(v) => unsafe { core::mem::transmute(v) },
                    UInt(v) => v,
                    Char(v) => v as u32,
                    String(ptr) => ptr.to_raw(),
                };
                self.write_to_buffer(raw)?;
            }
        }
        Ok(())
    }

    fn interface_result<T>(result: core::result::Result<T, I::Error>) -> Result<T, M, I, W> {
        match result {
            Ok(v) => Ok(v),
            Err(err) => Err(Error::Interface(err)),
        }
    }
}

impl<M: Model, I: Interface> Coprocessor<M, I, PollingWaiter<M, I>> {
    /// Consumes the given interface and returns an interface to the
    /// coprocessor via the given interface, which will use busy-polling to
    /// wait when there isn't enough buffer space.
    ///
    /// If your platform allows you to detect the EVE coprocessor space
    /// interrupt then you might prefer to call `new` and pass a custom
    /// waiter that can put your main processor to sleep while waiting,
    /// for better power usage compared to the default busy-polling
    /// implementation.
    pub fn new_polling(ei: I) -> Result<Self, M, I, PollingWaiter<M, I>> {
        let w: PollingWaiter<M, I> = PollingWaiter::new();
        Self::new(ei, w)
    }
}

/// These methods are available only when working with a model that has a
/// coprocessor error message memory space.
impl<M, I, W> Coprocessor<M, I, W>
where
    M: Model + crate::models::WithCommandErrMem,
    I: Interface,
    W: Waiter<M, I>,
{
    /// Returns the fault message currently available in the EVE coprocessor's
    /// fault message memory space.
    ///
    /// It's only meaningful to call this immediately after another coprocessor
    /// method returns the error variant `Fault`, before submitting any other
    /// coprocessor commands.
    ///
    /// The format of the returned message is determined entirely by the
    /// EVE chip, though it is typically a sequence of bytes representing an
    /// ASCII string.
    pub fn coprocessor_fault_msg(&mut self) -> Result<FaultMessage<M::CommandErrMem>, M, I, W> {
        use crate::memory::{CommandErrMem, MemoryRegion};
        use crate::models::WithCommandErrMem;

        let stopped = self.stop_stream()?;
        let mut raw = <<M as WithCommandErrMem>::CommandErrMem as CommandErrMem>::RawMessage::new();
        {
            let into = raw.as_storage_bytes();
            let ll = self.borrow_low_level(&stopped);
            let addr = <<M as WithCommandErrMem>::CommandErrMem as MemoryRegion>::ptr(0);
            Self::interface_result(ll.rd8s(addr, into))?;
        }
        self.start_stream(stopped)?;
        Ok(FaultMessage::new(raw))
    }
}

/// These methods are available only when working with a model that allows
/// _selecting_ coprocessor API level 1.
///
/// That doesn't include the BT815 and BT816 models: even though they do
/// support the API now referred to as API level 1, they don't support
/// _selecting_ that version because the idea of API levels was introduced
/// only with the BT817 and BT818 models.
impl<M, I, W> Coprocessor<M, I, W>
where
    M: Model + crate::models::WithCommandErrMem,
    I: Interface,
    W: Waiter<M, I>,
{
    /// On models that support multiple API levels, this selects API level 1
    /// which aims to be backward-compatible with the BT815 and BT816 models.
    pub fn use_api_level_1(&mut self) -> Result<(), M, I, W> {
        self.write_stream(8, |cp| {
            cp.write_to_buffer(0xFFFFFF63 as u32)?;
            cp.write_to_buffer(1 as u32)
        })
    }
}

/// These methods are available only when working with a model that allows
/// _selecting_ coprocessor API level 2.
impl<M, I, W> Coprocessor<M, I, W>
where
    M: Model + crate::models::WithCommandErrMem,
    I: Interface,
    W: Waiter<M, I>,
{
    /// On models that support multiple API levels, this selects API level 1
    /// which aims to be backward-compatible with the BT815 and BT816 models.
    ///
    /// There is no type-system-level enforcement that API level 2 is enabled
    /// before using API level 2 features, as a measure of pragmatism to
    /// avoid making the typestates of `Coprocessor` even more complex than
    /// they already are. If you try to use API level 2 features without
    /// first calling this method then the resulting behavior is undefined.
    pub fn use_api_level_2(&mut self) -> Result<(), M, I, W> {
        self.write_stream(8, |cp| {
            cp.write_to_buffer(0xFFFFFF63 as u32)?;
            cp.write_to_buffer(2 as u32)
        })
    }
}

/// Error type for coprocessor operations.
///
/// This distinguishes between errors from the underlying interface to the
/// hardware, errors returned by the "waiter" while waiting for more buffer
/// space, and coprocessor faults reported by the EVE chip itself.
#[derive(Debug)]
pub enum Error<IErr, WErr> {
    /// Errors encountered when sending or recieving data from the EVE chip.
    ///
    /// The wrapped error type for this variant is the error type for whichever
    /// [`Interface`](crate::Interface) implementation you are using.
    Interface(IErr),

    /// Errors encountered while waiting for more space in the ring buffer.
    ///
    /// The wrapped error type for this variant is the error type for whichever
    /// [`Waiter`](super::waiter::Waiter) implementation you are using. If you
    /// are using the default polling waiter then the error will be of the
    /// error type associated with your chosen [`Interface`](crate::Interface).
    Waiter(WErr),

    /// Indicates that the coprocessor itself reported a fault.
    ///
    /// If you are using an EVE chip that supports fault messages, you can call
    /// [`Coprocessor::coprocessor_fault_msg`](Coprocessor::coprocessor_fault_msg)
    /// to get an error string from the EVE chip.
    ///
    /// The coprocessor typically runs asynchronously from the host processor,
    /// and so a fault error may be returned from some later method call than
    /// the one which caused the fault. This error variant therefore indicates
    /// only that the coprocessor is blocked by being the fault state, not that
    /// the most recent method call put it in that state.
    Fault,
}

/// Represents a coprocessor fault message retrieved from the EVE device.
#[derive(Debug, Clone)]
pub struct FaultMessage<R: crate::memory::CommandErrMem>(R::RawMessage);

impl<R: crate::memory::CommandErrMem> FaultMessage<R> {
    fn new(raw: R::RawMessage) -> Self {
        Self(raw)
    }

    pub fn as_bytes<'a>(&'a self) -> &'a [u8] {
        self.0.as_bytes()
    }
}

#[doc(hide)]
pub trait FaultMessageRaw {
    fn new() -> Self;
    fn as_bytes<'a>(&'a self) -> &'a [u8];
    fn as_storage_bytes<'a>(&'a mut self) -> &'a mut [u8];
}

// [u8; 128] is the only raw type currently used by any models. We might need
// to add more of these if other models use different lengths in the future.
impl FaultMessageRaw for [u8; 128] {
    fn new() -> Self {
        [0; 128]
    }

    fn as_bytes<'a>(&'a self) -> &'a [u8] {
        // There should be a null terminator somewhere in the raw buffer,
        // which marks how long our returned slice ought to be.
        let all = &self[..];
        for (i, b) in all.iter().enumerate() {
            if *b == 0 {
                return &all[0..i];
            }
        }
        // We shouldn't get down here for a valid error string, but we'll
        // be robust and just return the whole "string" in that case.
        all
    }

    fn as_storage_bytes<'a>(&'a mut self) -> &'a mut [u8] {
        &mut self[..]
    }
}

// This type is used to create a zero-cost token representing codepaths in
// the Coprocessor type where the stream is stopped, to help ensure correct
// discipline around which functions expect to be called with the stream
// deactivated. It's an empty struct because its is only present for the
// type checker, not relevant at runtime.
struct StoppedStream;

impl<M, I, W> crate::display_list::Builder for Coprocessor<M, I, W>
where
    M: Model,
    I: Interface,
    W: Waiter<M, I>,
{
    type Error = Error<I::Error, W::Error>;

    fn append_raw_command(&mut self, raw: u32) -> core::result::Result<(), Self::Error> {
        self.append_raw_word(raw)
    }

    fn append_command(
        &mut self,
        cmd: crate::display_list::DLCmd,
    ) -> core::result::Result<(), Self::Error> {
        self.append_display_list(cmd)
    }
}

fn maybe_opt_format<R: crate::memory::MainMem>(
    given: u32,
    msg: &strfmt::Message<'_, '_, R>,
) -> u32 {
    const OPT_FORMAT: u32 = 4096;
    if msg.needs_format() {
        given | OPT_FORMAT
    } else {
        given
    }
}
