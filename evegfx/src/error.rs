//! Various error types returned by different components in this crate.

/// A general error type for errors from the main [`EVE`](crate::EVE) type,
/// and some other types such as [`LowLevel`](crate::low_level::LowLevel).
#[non_exhaustive]
pub enum Error<I: crate::interface::Interface> {
    /// Indicates that the requested operation isn't supported for the
    /// current model.
    ///
    /// The crate API is designed to handle certain model differences at
    /// compile time within the type system, but for reasons of pragmatism
    /// some differences are handled only dynamically.
    Unsupported,

    /// Errors encountered when sending or recieving data from the EVE chip.
    ///
    /// The wrapped error type for this variant is the error type for whichever
    /// [`Interface`](crate::interface::Interface) implementation you are using.
    Interface(I::Error),
}

impl<I> core::fmt::Debug for Error<I>
where
    I: crate::interface::Interface,
    I::Error: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::result::Result<(), core::fmt::Error> {
        match (&*self,) {
            (&Error::Interface(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Interface");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&Error::Unsupported,) => {
                let mut debug_trait_builder = f.debug_tuple("Unsupported");
                debug_trait_builder.finish()
            }
        }
    }
}

/// Error type for coprocessor operations.
///
/// This distinguishes between errors from the underlying interface to the
/// hardware, errors returned by the "waiter" while waiting for more buffer
/// space, and coprocessor faults reported by the EVE chip itself.
#[non_exhaustive]
pub enum CoprocessorError<M, I, W>
where
    M: crate::models::Model,
    I: crate::interface::Interface,
    W: crate::commands::waiter::Waiter<M, I>,
{
    /// Indicates that the requested operation isn't supported for the
    /// current model.
    ///
    /// The crate API is designed to handle certain model differences at
    /// compile time within the type system, but for reasons of pragmatism
    /// some differences are handled only dynamically.
    Unsupported,

    /// Errors encountered when sending or recieving data from the EVE chip.
    ///
    /// The wrapped error type for this variant is the error type for whichever
    /// [`Interface`](crate::interface::Interface) implementation you are using.
    Interface(I::Error),

    /// Errors encountered while waiting for more space in the ring buffer.
    ///
    /// The wrapped error type for this variant is the error type for whichever
    /// [`Waiter`](crate::commands::waiter::Waiter) implementation you are using. If you
    /// are using the default polling waiter then the error will be of the
    /// error type associated with your chosen [`Interface`](crate::interface::Interface).
    Waiter(W::Error),

    /// Indicates that the coprocessor itself reported a fault.
    ///
    /// If you are using an EVE chip that supports fault messages, you can call
    /// [`Coprocessor::coprocessor_fault_msg`](crate::commands::Coprocessor::coprocessor_fault_msg)
    /// to get an error string from the EVE chip.
    ///
    /// The coprocessor typically runs asynchronously from the host processor,
    /// and so a fault error may be returned from some later method call than
    /// the one which caused the fault. This error variant therefore indicates
    /// only that the coprocessor is blocked by being the fault state, not that
    /// the most recent method call put it in that state.
    Fault,
}

impl<M, I, W> CoprocessorError<M, I, W>
where
    M: crate::models::Model,
    I: crate::interface::Interface,
    W: crate::commands::waiter::Waiter<M, I>,
{
    pub fn from_general_error(err: Error<I>) -> Self {
        match err {
            Error::Unsupported => CoprocessorError::Unsupported,
            Error::Interface(e) => CoprocessorError::Interface(e),
        }
    }

    pub fn general_result<R>(r: Result<R, Error<I>>) -> Result<R, Self> {
        r.map_err(|e| Self::from_general_error(e))
    }
}

impl<M, I, W> core::fmt::Debug for CoprocessorError<M, I, W>
where
    M: crate::models::Model,
    I: crate::interface::Interface,
    W: crate::commands::waiter::Waiter<M, I>,
    I::Error: core::fmt::Debug,
    W::Error: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::result::Result<(), core::fmt::Error> {
        match (&*self,) {
            (&CoprocessorError::Unsupported,) => {
                let mut debug_trait_builder = f.debug_tuple("Unsupported");
                debug_trait_builder.finish()
            }
            (&CoprocessorError::Interface(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Interface");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&CoprocessorError::Waiter(ref __self_0),) => {
                let mut debug_trait_builder = f.debug_tuple("Waiter");
                let _ = debug_trait_builder.field(&&(*__self_0));
                debug_trait_builder.finish()
            }
            (&CoprocessorError::Fault,) => {
                let mut debug_trait_builder = f.debug_tuple("Fault");
                debug_trait_builder.finish()
            }
        }
    }
}

impl<M, I, W> From<Error<I>> for CoprocessorError<M, I, W>
where
    M: crate::models::Model,
    I: crate::interface::Interface,
    W: crate::commands::waiter::Waiter<M, I>,
{
    fn from(err: Error<I>) -> Self {
        Self::from_general_error(err)
    }
}
