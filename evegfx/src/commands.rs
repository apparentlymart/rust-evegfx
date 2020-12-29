use crate::low_level::LowLevel;
use crate::registers::EVERegister;
use crate::Interface;

/// An interface to the command ring buffer for the EVE chip's coprocessor
/// component.
///
/// This object encapsulates the handling of the ring buffer and provides an
/// API for appending
pub struct EVECoprocessor<I: Interface, W: EVECoprocessorWaiter<I>> {
    ll: LowLevel<I>,
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

impl<I: Interface, W: EVECoprocessorWaiter<I>> EVECoprocessor<I, W> {
    /// Consumes the given interface and waiter and returns an interface to
    /// the coprocessor via the given interface.
    ///
    /// This function consumes the interface because it will be constantly
    /// writing into the command ring buffer of the associated EVE chip and
    /// so it isn't safe to do any other concurrent access. You can get
    /// the underlying interface back again if you need it using some of
    /// the methods of `EVECoprocessor`.
    pub fn new(ei: I, wait: W) -> Result<Self, EVECoprocessorError<Self>> {
        let mut ll = crate::low_level::LowLevel::new(ei);

        // We'll pulse the reset signal for the coprocessor just to make sure
        // we're finding it in a known good state.
        Self::interface_result(ll.wr8(EVERegister::CPURESET.into(), 0b001))?;
        Self::interface_result(ll.wr8(EVERegister::CPURESET.into(), 0b000))?;

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
    pub fn with_new_waiter<W2, F>(self, f: F) -> EVECoprocessor<I, W2>
    where
        W2: EVECoprocessorWaiter<I>,
        F: FnOnce(W) -> W2,
    {
        let ll = self.ll;
        let old_wait = self.wait;
        let old_known_space = self.known_space;

        let new_wait = f(old_wait);

        EVECoprocessor {
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
    pub fn take_interface(mut self) -> Result<I, EVECoprocessorError<Self>> {
        self.stop_stream()?;
        return Ok(self.ll.take_interface());
    }

    /// `with_interface` runs your given closure with access to the
    /// coprocessor object's underlying `Interface`, temporarily pausing
    /// local coprocessor management so the closure can make use of other
    /// chip functionality.
    pub fn with_interface<R, F: FnOnce(&mut I) -> Result<R, EVECoprocessorError<Self>>>(
        &mut self,
        f: F,
    ) -> Result<R, EVECoprocessorError<Self>> {
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
    fn synchronize(&mut self, _stopped: &StoppedStream) -> Result<(), EVECoprocessorError<Self>> {
        let known_space = Self::interface_result(self.ll.rd16(EVERegister::CMDB_SPACE.into()))?;
        self.known_space = known_space;
        Ok(())
    }

    fn borrow_interface<'a>(&'a mut self, stopped: &StoppedStream) -> &'a mut I {
        let ll = self.borrow_low_level(stopped);
        ll.borrow_interface()
    }

    fn borrow_low_level<'a>(&'a mut self, _stopped: &StoppedStream) -> &'a mut LowLevel<I> {
        &mut self.ll
    }

    fn borrow_low_level_and_waiter<'a>(
        &'a mut self,
        _stopped: &StoppedStream,
    ) -> (&'a mut LowLevel<I>, &'a mut W) {
        (&mut self.ll, &mut self.wait)
    }

    // `start_stream` consumes the StoppedStream token because by the time it
    // returns the stream isn't stopped anymore.
    fn start_stream(&mut self, stopped: StoppedStream) -> Result<(), EVECoprocessorError<Self>> {
        // We now begin a write transaction at the next offset, so subsequent
        // command writes can just go directly into that active transaction.
        // This relies on the fact that EVE has a special behavior where
        // writes into RAM_CMD wrap around only inside the command space, not
        // in the whole memory space, and so we can just keep writing and let
        // the chip worry about the wraparound for us.
        let ei = self.borrow_interface(&stopped);
        Self::interface_result(ei.begin_write(EVERegister::CMDB_WRITE.address()))
    }

    // `stop_stream` produces a StoppedStream token to represent that it has
    // stopped the stream and thus the caller can safely perform operations
    // that expect the stream to be stopped.
    fn stop_stream(&mut self) -> Result<StoppedStream, EVECoprocessorError<Self>> {
        // This just closes the long-lived write transaction we started in
        // start_stream.
        let ei = self.ll.borrow_interface();
        Self::interface_result(ei.end_write())?;
        Ok(StoppedStream)
    }

    fn write_stream<F: FnOnce(&mut Self) -> Result<(), EVECoprocessorError<Self>>>(
        &mut self,
        len: u16,
        f: F,
    ) -> Result<(), EVECoprocessorError<Self>> {
        self.ensure_space(len)?;

        // We just assume that our stream will always be active here and
        // so we can just burst writes into it. It's the responsibility of
        // any function that takes actions other than writing into the FIFO
        // to temporarily stop and then restart the stream.EVERegister
        f(self)?;

        Ok(())
    }

    // Block using our waiter until there's at least `need` bytes of free space
    // in the ring buffer.
    fn ensure_space(&mut self, need: u16) -> Result<(), EVECoprocessorError<Self>> {
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
        {
            let (ll, wait) = self.borrow_low_level_and_waiter(&stopped);
            let known_space = Self::waiter_result(wait.wait_for_space(ll, need))?;
            self.known_space = known_space;
        }
        self.start_stream(stopped)?;
        Ok(())
    }

    // Write directly to the output stream. This function doesn't check whether
    // there's sufficient space in the buffer, so the caller should call
    // ensure_space first to wait until there's enough space for the full
    // message it intends to write.
    fn write_to_buffer(&mut self, v: u32) -> Result<(), EVECoprocessorError<Self>> {
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

    fn interface_result<T>(result: Result<T, I::Error>) -> Result<T, EVECoprocessorError<Self>> {
        match result {
            Ok(v) => Ok(v),
            Err(err) => Err(EVECoprocessorError::Interface(err)),
        }
    }

    fn waiter_result<T>(result: Result<T, W::Error>) -> Result<T, EVECoprocessorError<Self>> {
        match result {
            Ok(v) => Ok(v),
            Err(err) => Err(EVECoprocessorError::Waiter(err)),
        }
    }
}

impl<I: Interface> EVECoprocessor<I, PollingCoprocessorWaiter<I>> {
    /// Consumes the given interface and returns an interface to the
    /// coprocessor via the given interface, which will use busy-polling to
    /// wait when there isn't enough buffer space.
    ///
    /// If your platform allows you to detect the EVE coprocessor space
    /// interrupt then you might prefer to call `new` and pass a custom
    /// waiter that can put your main processor to sleep while waiting,
    /// for better power usage compared to the default busy-polling
    /// implementation.
    pub fn new_polling(ei: I) -> Result<Self, EVECoprocessorError<Self>> {
        let w: PollingCoprocessorWaiter<I> = PollingCoprocessorWaiter::new();
        Self::new(ei, w)
    }
}

/// The methods which submit new commands into the coprocessor ringbuffer.
///
/// These will block using the waiter if they run out of coprocessor buffer
/// space, but they will not wait if there's enough buffer space available to
/// write into.
impl<I: Interface, W: EVECoprocessorWaiter<I>> EVECoprocessor<I, W> {
    /// A convenience function for enclosing a series of coprocessor commands
    /// in `start_display_list` and `display_list_swap` commands.
    ///
    /// The given closure can in principle call all of the same methods as
    /// directly on the coprocessor object, but it's best to avoid any action
    /// that interacts with anything outside of the coprocessor. It _definitely_
    /// doesn't make sense to recursively call into `new_display_list` again.
    pub fn new_display_list<F>(&mut self, f: F) -> Result<(), EVECoprocessorError<Self>>
    where
        F: FnOnce(&mut Self) -> Result<(), EVECoprocessorError<Self>>,
    {
        self.start_display_list()?;
        f(self)?;
        self.display_list_swap()
    }

    pub fn show_testcard(&mut self) -> Result<(), EVECoprocessorError<Self>> {
        self.write_stream(4, |cp| cp.write_to_buffer(0xFFFFFF61))
    }

    pub fn show_manufacturer_logo(&mut self) -> Result<(), EVECoprocessorError<Self>> {
        self.write_stream(4, |cp| cp.write_to_buffer(0xFFFFFF31))
    }

    pub fn start_spinner(&mut self) -> Result<(), EVECoprocessorError<Self>> {
        // TODO: Make the spinner customizable.
        self.write_stream(20, |cp| {
            cp.write_to_buffer(0xFFFFFF16)?;
            cp.write_to_buffer(1000)?;
            cp.write_to_buffer(1000)?;
            cp.write_to_buffer(0)?;
            cp.write_to_buffer(0)
        })
    }

    pub fn start_display_list(&mut self) -> Result<(), EVECoprocessorError<Self>> {
        self.write_stream(4, |cp| cp.write_to_buffer(0xFFFFFF00))
    }

    pub fn display_list_swap(&mut self) -> Result<(), EVECoprocessorError<Self>> {
        self.write_stream(4, |cp| cp.write_to_buffer(0xFFFFFF01))
    }

    pub fn append_display_list(
        &mut self,
        cmd: crate::display_list::DLCmd,
    ) -> Result<(), EVECoprocessorError<Self>> {
        self.write_stream(4, |cp| cp.write_to_buffer(cmd.as_raw()))
    }

    pub fn append_raw_word(&mut self, word: u32) -> Result<(), EVECoprocessorError<Self>> {
        self.write_stream(4, |cp| cp.write_to_buffer(word))
    }

    pub fn wait_microseconds(&mut self, delay: u32) -> Result<(), EVECoprocessorError<Self>> {
        self.write_stream(8, |cp| {
            cp.write_to_buffer(0xFFFFFF65)?;
            cp.write_to_buffer(delay)
        })
    }

    pub fn wait_video_scanout(&mut self) -> Result<(), EVECoprocessorError<Self>> {
        self.write_stream(4, |cp| cp.write_to_buffer(0xFFFFFF42))
    }
}

/// The methods which block until the coprocessor has "caught up" with
/// particular events.
///
/// These make use of the associated "waiter" to block until specific
/// coprocessor commands have completed, and so applications making heavy
/// use of these may wish to consider supplying a tailored waiter
/// implementation that can avoid busy-waiting.
impl<I: Interface, W: EVECoprocessorWaiter<I>> EVECoprocessor<I, W> {
    /// Blocks until the coprocessor buffer is empty, signalling that the
    /// coprocessor has completed all of the commands issued so far.
    pub fn block_until_idle(&mut self) -> Result<(), EVECoprocessorError<Self>> {
        self.ensure_space(4092)
    }

    /// Blocks until EVE has finished scanning out the current frame. Callers
    /// can use this as part of a main loop which takes actions synchronized
    /// with the video framerate.
    ///
    /// This is a blocking version of `wait_video_scanout`.
    pub fn block_until_video_scanout(&mut self) -> Result<(), EVECoprocessorError<Self>> {
        self.wait_video_scanout()?;
        self.block_until_idle()
    }
}

// This type is used to create a zero-cost token representing codepaths in
// the EVECoprocessor type where the stream is stopped, to help ensure correct
// discipline around which functions expect to be called with the stream
// deactivated. It's an empty struct because its is only present for the
// type checker, not relevant at runtime.
struct StoppedStream;

impl<I, W> crate::display_list::EVEDisplayListBuilder for EVECoprocessor<I, W>
where
    I: Interface,
    W: EVECoprocessorWaiter<I>,
{
    type Error = EVECoprocessorError<Self>;

    fn append_raw_command(
        &mut self,
        raw: u32,
    ) -> core::result::Result<(), EVECoprocessorError<Self>> {
        self.append_raw_word(raw)
    }

    fn append_command(
        &mut self,
        cmd: crate::display_list::DLCmd,
    ) -> core::result::Result<(), EVECoprocessorError<Self>> {
        self.append_display_list(cmd)
    }
}

impl<I: Interface, W: EVECoprocessorWaiter<I>> Errorer for EVECoprocessor<I, W> {
    type InterfaceError = I::Error;
    type WaiterError = W::Error;
}

/// A `CoprocessorWaiter` is an object that knows how to block until the
/// coprocessor ring buffer is at least empty enough to receive a forthcoming
/// message.
///
/// This is a trait in order to allow for implementations that are able to
/// respond to the EVE's interrupt signal for the buffer to be ready, although
/// the only implementation available directly in this crate is one that
/// busy-polls the register that tracks the buffer usage, because interaction
/// with interrupts is always system-specific.
pub trait EVECoprocessorWaiter<I: Interface> {
    type Error;

    fn wait_for_space(&mut self, ell: &mut LowLevel<I>, need: u16) -> Result<u16, Self::Error>;
}

#[derive(Debug)]
pub enum EVECoprocessorError<Emitter: Errorer> {
    Interface(Emitter::InterfaceError),
    Waiter(Emitter::WaiterError),
}

/// Implemented by types that can produce `EVECoprocessorError` errors.
pub trait Errorer {
    type InterfaceError;
    type WaiterError;
}

pub(crate) struct PollingCoprocessorWaiter<I: Interface> {
    _ei: core::marker::PhantomData<I>,
}

impl<I: Interface> PollingCoprocessorWaiter<I> {
    fn new() -> Self {
        Self {
            _ei: core::marker::PhantomData,
        }
    }
}

impl<I: Interface> EVECoprocessorWaiter<I> for PollingCoprocessorWaiter<I> {
    type Error = I::Error;

    fn wait_for_space(&mut self, ell: &mut LowLevel<I>, need: u16) -> Result<u16, Self::Error> {
        loop {
            let known_space = ell.rd16(EVERegister::CMDB_SPACE.into())?;
            if known_space >= need {
                return Ok(known_space);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::interface::{EVEAddress, EVECommand};
    use crate::registers::EVERegister::*;
    use std::vec;
    use std::vec::Vec;

    fn test_obj<F: FnOnce(&mut MockInterface)>(
        setup: F,
    ) -> EVECoprocessor<MockInterface, impl crate::commands::EVECoprocessorWaiter<MockInterface>>
    {
        let mut interface = MockInterface::new();
        setup(&mut interface);
        unwrap_copro(EVECoprocessor::new_polling(interface))
    }

    fn unwrap_copro<R, W: EVECoprocessorWaiter<MockInterface>>(
        v: Result<R, EVECoprocessorError<EVECoprocessor<MockInterface, W>>>,
    ) -> R {
        match v {
            Ok(v) => v,
            Err(err) => match err {
                EVECoprocessorError::Interface(err) => {
                    std::panic!("interface error: {:?}", err);
                }
                EVECoprocessorError::Waiter(_) => {
                    std::panic!("waiter error");
                }
            },
        }
    }

    #[test]
    fn test_handle_buffer_space() {
        let mut cp = test_obj(|ei| {
            // For this test we'll make the available buffer space much
            // smaller than normal, so we can see the coprocessor object
            // handle what looks like running out of buffer space.
            ei.current_space = 16;
        });

        // Some junk data just to use up buffer space.
        unwrap_copro(cp.append_raw_word(0xf4ce0001));
        unwrap_copro(cp.append_raw_word(0xf4ce0002));
        unwrap_copro(cp.append_raw_word(0xf4ce0003));
        // We now have only four bytes left, but the following command needs
        // eight so we should ReadSpace before writing it.
        unwrap_copro(cp.wait_microseconds(127));

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(16),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xf4ce0001), // Junk data 1
            MockInterfaceCall::Write(0xf4ce0002), // Junk data 2
            MockInterfaceCall::Write(0xf4ce0003), // Junk data 3
            // The coprocessor object now only knows about four remaining
            // bytes of buffer space, but the next command requires eight and
            // so we'll poll to see if more space is available before
            // continuing.
            MockInterfaceCall::StopStream,
            MockInterfaceCall::ReadSpace(16),
            MockInterfaceCall::StartStream,
            // There is now >= 8 bytes buffer space, so we can continue with
            // appending the two words of the wait_microseconds command.
            MockInterfaceCall::Write(0xffffff65), // CMD_WAIT
            MockInterfaceCall::Write(127),        // the duration value from above
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_new_display_list() {
        let mut cp = test_obj(|_| {});

        unwrap_copro(cp.new_display_list(|cp| {
            // We must import this trait in order to call the display list building
            // methods on the coprocessor object.
            use crate::display_list::EVEDisplayListBuilder;

            cp.append_raw_word(0xdeadbeef)?;
            cp.clear_color_alpha(127)
        }));

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xffffff00), // CMD_DLSTART
            MockInterfaceCall::Write(0xdeadbeef), // Fake display list command
            MockInterfaceCall::Write(0x0f00007f), // CLEAR_COLOR_A(127)
            MockInterfaceCall::Write(0xffffff01), // CMD_SWAP
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_show_testcard() {
        let mut cp = test_obj(|_| {});

        unwrap_copro(cp.show_testcard());

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xffffff61), // CMD_TESTCARD
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_show_manufacturer_logo() {
        let mut cp = test_obj(|_| {});

        unwrap_copro(cp.show_manufacturer_logo());

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xffffff31), // CMD_LOGO
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_wait_microseconds() {
        let mut cp = test_obj(|_| {});

        unwrap_copro(cp.wait_microseconds(12345));

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xffffff65), // CMD_WAIT
            MockInterfaceCall::Write(12345),      // the duration value from above
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    /// A test double for `trait Interface`, available only in test mode.
    pub struct MockInterface {
        write_addr: Option<EVEAddress>,
        read_addr: Option<EVEAddress>,

        pub(crate) current_space: u16,

        // calls_ is the call log. Each call to a mock method appends one
        // entry to this vector, including any that fail.
        calls_: Vec<MockInterfaceCall>,
    }

    #[derive(Clone)]
    pub enum MockInterfaceCall {
        ReadSpace(u16),
        Write(u32),
        StartStream,
        StopStream,
    }

    impl std::fmt::Debug for MockInterfaceCall {
        fn fmt(
            &self,
            f: &mut core::fmt::Formatter<'_>,
        ) -> core::result::Result<(), core::fmt::Error> {
            match self {
                MockInterfaceCall::ReadSpace(space) => write!(f, "ReadSpace({:#4?})", space),
                MockInterfaceCall::Write(v) => write!(f, "Write({:#010x?})", v),
                MockInterfaceCall::StartStream => write!(f, "StartStream"),
                MockInterfaceCall::StopStream => write!(f, "StopStream"),
            }
        }
    }

    impl MockInterface {
        const SPACE_ADDR: EVEAddress = CMDB_SPACE.address();
        const WRITE_ADDR: EVEAddress = CMDB_WRITE.address();

        pub fn new() -> Self {
            Self {
                write_addr: None,
                read_addr: None,
                current_space: 0xffc,
                calls_: Vec::new(),
            }
        }

        /// Consumes the mock and returns all of the calls it logged
        /// during its life.
        pub fn calls(self) -> Vec<MockInterfaceCall> {
            self.calls_
        }
    }

    #[derive(Debug)]
    pub struct MockError(&'static str);

    impl Interface for MockInterface {
        type Error = MockError;

        fn begin_write(&mut self, addr: EVEAddress) -> core::result::Result<(), Self::Error> {
            if let Some(_) = self.write_addr {
                return Err(MockError("begin_write when a write is already active"));
            }
            if let Some(_) = self.read_addr {
                return Err(MockError("begin_write when a read is already active"));
            }
            if addr == Self::WRITE_ADDR {
                self.calls_.push(MockInterfaceCall::StartStream);
            }
            if addr == Self::SPACE_ADDR {
                return Err(MockError("mustn't write to REG_CMDB_SPACE"));
            }
            self.write_addr = Some(addr);
            Ok(())
        }

        fn continue_write(&mut self, buf: &[u8]) -> core::result::Result<(), Self::Error> {
            match self.write_addr {
                Some(addr) => {
                    if addr == Self::WRITE_ADDR {
                        if buf.len() != 4 {
                            return Err(MockError("must write to REG_CMDB_WRITE using wr32"));
                        }
                        let v = (buf[0] as u32)
                            | (buf[1] as u32) << 8
                            | (buf[2] as u32) << 16
                            | (buf[3] as u32) << 24;
                        self.calls_.push(MockInterfaceCall::Write(v));
                    }
                    // We ignore all other writes because they aren't relevant
                    // to our coprocessor testing.
                    Ok(())
                }
                None => Err(MockError("continue_write without an active write")),
            }
        }

        fn end_write(&mut self) -> core::result::Result<(), Self::Error> {
            let result = match self.write_addr {
                Some(addr) => {
                    if addr == Self::WRITE_ADDR {
                        self.calls_.push(MockInterfaceCall::StopStream);
                    }
                    // We ignore all other addresses because they aren't relevant
                    // to our coprocessor testing.
                    Ok(())
                }
                None => Err(MockError("end_write without an active write")),
            };
            self.write_addr = None;
            result
        }

        fn begin_read(&mut self, addr: EVEAddress) -> core::result::Result<(), Self::Error> {
            if let Some(_) = self.write_addr {
                return Err(MockError("begin_read when a write is already active"));
            }
            if let Some(_) = self.read_addr {
                return Err(MockError("begin_read when a read is already active"));
            }
            if addr == Self::WRITE_ADDR {
                return Err(MockError("mustn't read from REG_CMDB_WRITE"));
            }
            self.read_addr = Some(addr);
            Ok(())
        }

        fn continue_read(&mut self, into: &mut [u8]) -> core::result::Result<(), Self::Error> {
            match self.read_addr {
                Some(addr) => {
                    if addr == Self::SPACE_ADDR {
                        if into.len() != 2 {
                            return Err(MockError("must read REG_CMDB_SPACE with rd16"));
                        }
                        self.calls_
                            .push(MockInterfaceCall::ReadSpace(self.current_space));
                        into[0] = (self.current_space & 0xff) as u8;
                        into[1] = (self.current_space >> 8) as u8;
                    }
                    // We ignore all other writes because they aren't relevant
                    // to our coprocessor testing.
                    Ok(())
                }
                None => Err(MockError("continue_read without an active read")),
            }
        }

        fn end_read(&mut self) -> core::result::Result<(), Self::Error> {
            let result = match self.read_addr {
                Some(_) => Ok(()),
                None => Err(MockError("end_read without an active read")),
            };
            self.read_addr = None;
            result
        }

        fn cmd(
            &mut self,
            _cmd: EVECommand,
            _a0: u8,
            _a1: u8,
        ) -> core::result::Result<(), Self::Error> {
            // Commands aren't relevant to our coprocessor tests.
            Ok(())
        }
    }

    impl PartialEq for MockInterfaceCall {
        fn eq(&self, other: &Self) -> bool {
            match self {
                MockInterfaceCall::ReadSpace(self_space) => {
                    if let MockInterfaceCall::ReadSpace(other_space) = other {
                        *self_space == *other_space
                    } else {
                        false
                    }
                }
                MockInterfaceCall::Write(self_data) => {
                    if let MockInterfaceCall::Write(other_data) = other {
                        self_data.eq(other_data)
                    } else {
                        false
                    }
                }
                MockInterfaceCall::StartStream => {
                    if let MockInterfaceCall::StartStream = other {
                        true
                    } else {
                        false
                    }
                }
                MockInterfaceCall::StopStream => {
                    if let MockInterfaceCall::StopStream = other {
                        true
                    } else {
                        false
                    }
                }
            }
        }
    }
}
