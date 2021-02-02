/// A higher-level interface trait tailored to the needs of the coprocessor
/// API.
///
/// Although most of the time we access the coprocessor via the lower-level
/// [`Interface`](super::Interface), this separate layer is intended for
/// situations where application code is isolated from the actual transport
/// hardware and is instead interacting with the EVE coprocessor via a
/// system call interface, or similar. This might be the case if your
/// operating system or RTOS has a kernel driver for EVE, for example.
pub trait CommandInterface {
    type Error;

    /// Write the raw command words given in `cmds` to the EVE coprocessor,
    /// blocking if there isn't yet enough space in the coprocessor buffer
    /// or in any other intermediate buffer.
    fn write_commands(&mut self, cmds: impl IntoIterator<Item = u32>) -> Result<(), Self::Error>;

    /// Block for the coprocessor to complete all of the commands already
    /// written into its buffer, and then optionally capture one or more
    /// raw words from the most recently-written words backwards.
    ///
    /// The `results` slice is necessary for the coprocessor commands which
    /// return results by overwriting parts of their input in the coprocessor
    /// buffer. For those we must capture the relevant data from the buffer
    /// before we continue writing, so that newly-written commands won't
    /// overwrite the result data.
    fn wait(&mut self, results: &mut [u32]) -> Result<(), Self::Error>;
}

/// Returns a `CommandInterface` implementation which submits commands
/// directly to the given lower-level [`Interface`](super::Interface), using
/// the given [`Waiter`](crate::commands::waiter::Waiter) to block for
/// necessary space in the command buffer.
pub fn direct_command_interface<M: crate::models::Model, I: super::Interface>(
    ei: I,
    waiter: impl crate::commands::waiter::Waiter<M, I>,
) -> impl CommandInterface {
}

struct DirectCommandInterface<
    M: crate::models::Model,
    I: super::Interface,
    W: crate::commands::waiter::Waiter<M, I>,
> {}

impl<M: crate::models::Model, I: super::Interface, W: crate::commands::waiter::Waiter<M, I>>
    CommandInterface for DirectCommandInterface<M, I, W>
{
}
