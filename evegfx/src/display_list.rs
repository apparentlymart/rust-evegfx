/// Represents an EVE display list command.
#[derive(Copy, Clone, PartialEq)]
pub struct DLCmd(u32);

impl DLCmd {
    // The length of a display list command as stored in the EVE device's
    // display list RAM.
    pub const LENGTH: u32 = 4;

    /// Creates a command from the raw command word given as a `u32`. It's
    /// the caller's responsibility to ensure that it's a valid encoding of
    /// a real display list command.
    pub const fn raw(raw: u32) -> Self {
        Self(raw)
    }
}

/// Each command is encoded as a four-byte value. Converting to `u32` returns
/// the raw encoding of the command, as it would be written into display
/// list memory (endianness notwithstanding).
impl Into<u32> for DLCmd {
    fn into(self) -> u32 {
        self.0
    }
}
