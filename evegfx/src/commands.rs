use crate::low_level::EVELowLevel;
use crate::registers::EVERegister;
use crate::EVEInterface;

/// An interface to the command ring buffer for the EVE chip's coprocessor
/// component.
///
/// This object encapsulates the handling of the ring buffer and provides an
/// API for appending
pub struct EVECoprocessor<I: EVEInterface, W: EVECoprocessorWaiter<I>> {
    ll: EVELowLevel<I>,
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

impl<I: EVEInterface, W: EVECoprocessorWaiter<I>> EVECoprocessor<I, W> {
    /// Consumes the given interface and waiter and returns an interface to
    /// the coprocessor via the given interface.
    ///
    /// This function consumes the interface because it will be constantly
    /// writing into the command ring buffer of the associated EVE chip and
    /// so it isn't safe to do any other concurrent access. You can get
    /// the underlying interface back again if you need it using some of
    /// the methods of `EVECoprocessor`.
    pub fn new(ei: I, wait: W) -> Result<Self, EVECoprocessorError<Self>> {
        let mut ll = crate::low_level::EVELowLevel::new(ei);

        // We'll pulse the reset signal for the coprocessor just to make sure
        // we're finding it in a known good state.
        Self::interface_result(ll.wr8(EVERegister::CPURESET.into(), 0b001))?;
        Self::interface_result(ll.wr8(EVERegister::CPURESET.into(), 0b000))?;

        let mut ret = Self {
            ll: ll,
            wait: wait,
            known_space: 0,
        };

        // Copy the current values for free space and next offset into our
        // local fields so we will start off in sync with the remote chip.
        ret.synchronize()?;

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

    /// Blocks until the coprocessor buffer is empty, signalling that the
    /// coprocessor has completed all of the commands issued so far.
    pub fn block_until_idle(&mut self) -> Result<(), EVECoprocessorError<Self>> {
        self.ensure_space(4092)
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

    /// `take_interface` consumes the coprocessor object and returns its
    /// underlying `EVEInterface`.
    ///
    /// To make temporary use of the underlying interface, without also
    /// discarding the coprocessor object, use `with_interface` instead.
    pub fn take_interface(self) -> I {
        return self.ll.take_interface();
    }

    /// `with_interface` runs your given closure with access to the
    /// coprocessor object's underlying `EVEInterface`, temporarily pausing
    /// local coprocessor management so the closure can make use of other
    /// chip functionality.
    pub fn with_interface<R, F: FnOnce(&mut I) -> Result<R, EVECoprocessorError<Self>>>(
        &mut self,
        f: F,
    ) -> Result<R, EVECoprocessorError<Self>> {
        let result = {
            let ei = self.ll.borrow_interface();
            f(ei)
        };
        // The caller could've messed with the registers we depend on, so
        // we'll resynchronize them before we restart our write stream.
        self.synchronize()?;
        result
    }

    fn synchronize(&mut self) -> Result<(), EVECoprocessorError<Self>> {
        let known_space = Self::interface_result(self.ll.rd16(EVERegister::CMDB_SPACE.into()))?;
        self.known_space = known_space;
        Ok(())
    }

    fn start_stream(&mut self) -> Result<(), EVECoprocessorError<Self>> {
        // We now begin a write transaction at the next offset, so subsequent
        // command writes can just go directly into that active transaction.
        // This relies on the fact that EVE has a special behavior where
        // writes into RAM_CMD wrap around only inside the command space, not
        // in the whole memory space, and so we can just keep writing and let
        // the chip worry about the wraparound for us.
        let ei = self.ll.borrow_interface();
        //let next_offset = self.next_offset as u32;
        /*Self::interface_result(
            ei.begin_write(crate::interface::EVEAddressRegion::RAM_CMD.base + next_offset),
        )*/
        Self::interface_result(ei.begin_write(EVERegister::CMDB_WRITE.address()))
    }

    fn stop_stream(&mut self) -> Result<(), EVECoprocessorError<Self>> {
        // This just closes the long-lived write transaction we started in
        // start_stream.
        let ei = self.ll.borrow_interface();
        Self::interface_result(ei.end_write())
    }

    fn write_stream<F: FnOnce(&mut Self) -> Result<(), EVECoprocessorError<Self>>>(
        &mut self,
        len: u16,
        f: F,
    ) -> Result<(), EVECoprocessorError<Self>> {
        self.ensure_space(len)?;

        self.start_stream()?;
        f(self)?;
        self.stop_stream()?;

        Ok(())
    }

    // Block using our waiter until there's at least `need` bytes of free space
    // in the ring buffer.
    fn ensure_space(&mut self, need: u16) -> Result<(), EVECoprocessorError<Self>> {
        if self.known_space >= need {
            // Fast path: our local tracking knows there's enough space.
            return Ok(());
        }

        // Otherwise we need to use our waiter to block until there's
        // enough space, and then update our record of known_space in the
        // hope of using the fast path next time.
        let known_space = Self::waiter_result(self.wait.wait_for_space(&mut self.ll, need))?;
        self.known_space = known_space;
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

impl<I: EVEInterface> EVECoprocessor<I, PollingCoprocessorWaiter<I>> {
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

impl<I, W> crate::display_list::EVEDisplayListBuilder for EVECoprocessor<I, W>
where
    I: EVEInterface,
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

impl<I: EVEInterface, W: EVECoprocessorWaiter<I>> Errorer for EVECoprocessor<I, W> {
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
pub trait EVECoprocessorWaiter<I: EVEInterface> {
    type Error;

    fn wait_for_space(&mut self, ell: &mut EVELowLevel<I>, need: u16) -> Result<u16, Self::Error>;
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

pub(crate) struct PollingCoprocessorWaiter<I: EVEInterface> {
    _ei: core::marker::PhantomData<I>,
}

impl<I: EVEInterface> PollingCoprocessorWaiter<I> {
    fn new() -> Self {
        Self {
            _ei: core::marker::PhantomData,
        }
    }
}

impl<I: EVEInterface> EVECoprocessorWaiter<I> for PollingCoprocessorWaiter<I> {
    type Error = I::Error;

    fn wait_for_space(&mut self, ell: &mut EVELowLevel<I>, need: u16) -> Result<u16, Self::Error> {
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
    use crate::interface::testing::{MockInterface, MockInterfaceCall};
    use crate::registers::EVERegister::*;
    use std::vec;

    fn test_obj<F: FnOnce(&mut MockInterface)>(
        setup: F,
    ) -> EVECoprocessor<MockInterface, impl crate::commands::EVECoprocessorWaiter<MockInterface>>
    {
        let mut interface = MockInterface::new();
        setup(&mut interface);
        match EVECoprocessor::new_polling(interface) {
            Ok(v) => v,
            Err(_) => panic!("failed to construct test object"),
        }
    }

    fn assert_success<R, E>(v: Result<R, E>) -> R {
        match v {
            Ok(v) => v,
            Err(_) => panic!("call failed"),
        }
    }

    #[test]
    fn test_new_display_list() {
        let mut cp = test_obj(|_| {});

        assert_success(cp.new_display_list(|cp| cp.append_raw_word(0xdeadbeef)));

        let got = cp.take_interface().calls();
        let want = vec![
            // Initial reset
            MockInterfaceCall::BeginWrite(CPURESET.address()),
            MockInterfaceCall::ContinueWrite(vec![0x01 as u8]),
            MockInterfaceCall::EndWrite(CPURESET.address()),
            MockInterfaceCall::BeginWrite(CPURESET.address()),
            MockInterfaceCall::ContinueWrite(vec![0x00 as u8]),
            MockInterfaceCall::EndWrite(CPURESET.address()),
            // Initial synchronize
            MockInterfaceCall::BeginRead(CMDB_SPACE.address()),
            MockInterfaceCall::ContinueRead(2),
            MockInterfaceCall::EndRead(CMDB_SPACE.address()),
            // The new_display_list call
            MockInterfaceCall::BeginWrite(CMDB_WRITE.address()),
            MockInterfaceCall::ContinueWrite(vec![0x00, 0xff, 0xff, 0xff as u8]),
            MockInterfaceCall::EndWrite(CMDB_WRITE.address()),
            MockInterfaceCall::BeginWrite(CMDB_WRITE.address()),
            MockInterfaceCall::ContinueWrite(vec![239, 190, 173, 222 as u8]),
            MockInterfaceCall::EndWrite(CMDB_WRITE.address()),
            MockInterfaceCall::BeginWrite(CMDB_WRITE.address()),
            MockInterfaceCall::ContinueWrite(vec![0x01, 0xff, 0xff, 0xff as u8]),
            MockInterfaceCall::EndWrite(CMDB_WRITE.address()),
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }
}
