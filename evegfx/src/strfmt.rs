/// A text message that might contain formatting sequences that correspond with
/// given arguments.
///
/// The main way to construct a `Message` is using the macro
/// [`evegfx::eve_format!`](crate::eve_format), which understands the EVE
/// formatting syntax just enough to automatically infer the argument types
/// and verify that the arguments are compatible with the format string.
///
/// ```rust
/// let val = 5;
/// println!("Message is {:?}", evegfx::eve_format!("The current value is %d", val));
/// ```
///
/// `Message` can also represent messages that won't be formatted at all,
/// although in that case it behaves just as a thin wrapper around a slice
/// of bytes.
#[derive(Clone, Copy)]
pub struct Message<'a, 'b> {
    pub(crate) fmt: &'a [u8],
    pub(crate) args: Option<&'b [Argument]>,
}

impl<'a, 'b> core::fmt::Debug for Message<'a, 'b> {
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
pub enum Argument {
    Int(i32),
    UInt(u32),
    Char(char),
    // TODO: Also string pointer into RAM_G
}

impl<'a, 'b> Message<'a, 'b> {
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
    pub const fn new(fmt: &'a [u8], args: &'b [Argument]) -> Self {
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
    pub const fn new_literal(lit: &'a [u8]) -> Self {
        Self {
            fmt: lit,
            args: None,
        }
    }

    /// Returns true if the message should be used with the format option.
    pub const fn needs_format(&self) -> bool {
        if let Some(_) = self.args {
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;

    #[test]
    fn test_instantiate_directly_no_args() {
        let _got = Message::new(b"hi", &[]);
    }

    #[test]
    fn test_instantiate_directly_with_arg() {
        let _got = Message::new(b"%d", &[Argument::Int(3)]);
    }

    // We can't test the eve_format! macro directly here, because it
    // constructs absolute paths that can only work in _other_ crates.
}
