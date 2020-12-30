//! Helpers for waiting until the coprocessor has freed enough ring buffer
//! space for a forthcoming command.
//!
//! [`Waiter`](Waiter) is a trait implemented by types that are able to block
//! until there's either a particular amount of buffer space available or
//! until the coprocessor reports a fault.
//!
//! [`PollingWaiter`](PollingWaiter) is a simple built-in implementation of
//! `Waiter` which busy-polls the coprocessor registers.
//!
//! If you are working with this library on a platform where you are able to
//! listen for and respond to interrupt signals from the EVE chip then you
//! could improve power consumption by implementing a new `Waiter` which can
//! put the host processor to sleep while waiting for a signal that there is
//! either more buffer space or a coprocessor fault.

use crate::interface::Interface;
use crate::low_level::LowLevel;
use crate::models::Model;
use crate::registers::EVERegister;

/// Knows how to block until the coprocessor ring buffer is at least empty
/// enough to receive a forthcoming message.
///
/// This is a trait in order to allow for implementations that are able to
/// respond to the EVE's interrupt signal for the buffer to be ready, although
/// the only implementation available directly in this crate is one that
/// busy-polls the register that tracks the buffer usage, because interaction
/// with interrupts is always system-specific.
pub trait Waiter<M: Model, I: Interface> {
    type Error;

    fn wait_for_space(
        &mut self,
        ell: &mut LowLevel<M, I>,
        need: u16,
    ) -> core::result::Result<u16, WaiterError<Self::Error>>;
}

/// Error type returned by a waiter, which distinguishes between communication
/// transport errors and explicit coprocessor faults.
#[derive(Debug)]
pub enum WaiterError<E: Sized> {
    Comm(E),
    Fault,
}

fn waiter_comm_result<R, E: Sized>(
    result: core::result::Result<R, E>,
) -> core::result::Result<R, WaiterError<E>> {
    match result {
        Ok(v) => Ok(v),
        Err(err) => Err(WaiterError::Comm(err)),
    }
}

/// The default [`Waiter`](Waiter) implementation, which polls the coprocessor
/// registers in a busy loop until there's enough available space.
pub struct PollingWaiter<M: Model, I: Interface> {
    _ei: core::marker::PhantomData<I>,
    _m: core::marker::PhantomData<M>,
}

impl<M: Model, I: Interface> PollingWaiter<M, I> {
    pub(crate) fn new() -> Self {
        Self {
            _ei: core::marker::PhantomData,
            _m: core::marker::PhantomData,
        }
    }
}

impl<M: Model, I: Interface> Waiter<M, I> for PollingWaiter<M, I> {
    type Error = I::Error;

    fn wait_for_space(
        &mut self,
        ell: &mut LowLevel<M, I>,
        need: u16,
    ) -> core::result::Result<u16, WaiterError<Self::Error>> {
        loop {
            let known_space = waiter_comm_result(ell.rd16(ell.reg_ptr(EVERegister::CMDB_SPACE)))?;
            if (known_space % 4) != 0 {
                // An unaligned amount of space indicates a coprocessor fault.
                return Err(WaiterError::Fault);
            }
            if known_space >= need {
                return Ok(known_space);
            }
        }
    }
}
