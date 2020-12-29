use crate::memory::MainMem;

/// A text message that might contain formatting sequences that correspond with
/// given arguments.
///
/// The main way to construct a `Message` is using the macro
/// [`evegfx::eve_format!`](crate::eve_format), which understands the EVE
/// formatting syntax just enough to automatically infer the argument types
/// and verify that the arguments are compatible with the format string.
///
/// `Message` can also represent messages that won't be formatted at all,
/// although in that case it behaves just as a thin wrapper around a slice
/// of bytes.
#[derive(Clone, Copy)]
pub struct Message<'a, 'b, R: MainMem = NoMem> {
    pub(crate) fmt: &'a [u8],
    pub(crate) args: Option<&'b [Argument<R>]>,
}

impl<'a, 'b, R: MainMem> core::fmt::Debug for Message<'a, 'b, R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::result::Result<(), core::fmt::Error> {
        use core::fmt::Write;
        write!(f, "Message {{ fmt: b\"")?;
        for c in self.fmt.iter() {
            for c in core::ascii::escape_default(*c) {
                let c = c as char;
                f.write_char(c)?;
            }
        }
        write!(f, "\", args: {:?} }}", self.args)
    }
}

/// An argument used as part of a `Message`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Argument<R: MainMem> {
    Int(i32),
    UInt(u32),
    Char(char),
    String(crate::memory::Ptr<R>),
}

impl<'a, 'b, R: MainMem> Message<'a, 'b, R> {
    /// Construct a message with formatting arguments.
    ///
    /// This function doesn't verify that the format string is compatible with
    /// the given arguments. If the arguments are incompatible then a
    /// generated coprocessor command would be invalid.
    ///
    /// A `Message` object only borrows the format string and argument slice
    /// it refers to, so the caller must keep hold of those objects and
    /// keep them in scope for the lifetime of the `Message`. This type is
    /// intended only for transient use to temporarily lend the message data
    /// to some of the [`EVECoprocessor`](crate::commands::EVECoprocessor)
    /// methods, to copy the data into the coprocessor ring buffer.
    #[inline]
    pub fn new(fmt: &'a [u8], args: &'b [Argument<R>]) -> Self {
        Self {
            fmt: fmt,
            args: Some(args),
        }
    }

    /// Constructs a message that doesn't use the formatting functionality,
    /// and instead just renders literally.
    ///
    /// This function doesn't verify that the given message is valid. In
    /// particular, if you pass a literal containing any null bytes then a
    /// generated coprocessor commands would be invalid.
    ///
    /// A `Message` object only borrows the format string it refers to, so the
    /// caller must keep hold of those objects and keep them in scope for the
    /// lifetime of the `Message`. This type is intended only for transient use
    /// to temporarily lend the message data to some of the
    /// [`EVECoprocessor`](crate::commands::EVECoprocessor)
    /// methods, to copy the data into the coprocessor ring buffer.
    #[inline]
    pub fn new_literal(lit: &'a [u8]) -> Self {
        Self {
            fmt: lit,
            args: None,
        }
    }

    /// Returns true if the message should be used with the format option.
    pub fn needs_format(&self) -> bool {
        if let Some(_) = self.args {
            true
        } else {
            false
        }
    }
}

/// NoModel is a stand-in model for messages that don't refer to main memory
/// at all, and thus aren't constrained to any particular model.
#[doc(hidden)]
#[derive(Debug)]
pub struct NoModel;

impl crate::models::Model for NoModel {
    type MainMem = NoMem;
    type DisplayListMem = NoMem;
    type RegisterMem = NoMem;
    type CommandMem = NoMem;
}

/// NoMem is a stand-in memory region for messages that don't refer to
/// main memory at all.
#[doc(hidden)]
#[derive(Debug)]
pub enum NoMem {}

impl crate::memory::MemoryRegion for NoMem {
    type Model = NoModel;
    const BASE_ADDR: u32 = 0;
    const LENGTH: u32 = 0;
    const DEBUG_NAME: &'static str = "NoMem";
}
impl crate::memory::HostAccessible for NoMem {}
impl crate::memory::MainMem for NoMem {}
impl crate::memory::DisplayListMem for NoMem {}
impl crate::memory::RegisterMem for NoMem {}
impl crate::memory::CommandMem for NoMem {}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;

    #[test]
    fn test_instantiate_directly_no_args() {
        let _got: Message = Message::new(b"hi", &[]);
    }

    #[test]
    fn test_instantiate_directly_with_arg() {
        let _got: Message = Message::new(b"%d", &[Argument::Int(3)]);
    }

    // We can't test the eve_format! macro directly here, because it
    // constructs absolute paths that can only work in _other_ crates.
}
