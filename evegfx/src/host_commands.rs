/// Represents a register within the MEM_REG region on an EVE device.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum EVEHostCmd {
    ACTIVE = 0x00,
    STANDBY = 0x41,
    SLEEP = 0x42,
    PWRDOWN = 0x43,
    CLKEXT = 0x44,
    CLKINT = 0x48,
    CLKSEL = 0x61,
    RST_PULSE = 0x68,
    PINDRIVE = 0x70,
    PIN_PD_STATE = 0x71,
}

impl EVEHostCmd {
    /// Returns the representation of the command expected by the low level
    /// interface API.
    ///
    /// The low-level API accepts any command value that matches the expected
    /// bitmask for commands, regardless of whether it's a specific command
    /// value defined in a datasheet.
    pub const fn for_interface(self) -> crate::interface::EVECommand {
        crate::interface::EVECommand::force_raw(self as u8)
    }
}

impl core::convert::From<EVEHostCmd> for crate::interface::EVECommand {
    fn from(cmd: EVEHostCmd) -> Self {
        cmd.for_interface()
    }
}
