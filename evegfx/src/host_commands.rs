/// Represents a register within the MEM_REG region on an EVE device.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    pub const fn from_raw(raw: u8) -> Option<Self> {
        use EVEHostCmd::*;
        match raw {
            0x00 => Some(ACTIVE),
            0x41 => Some(STANDBY),
            0x42 => Some(SLEEP),
            0x43 => Some(PWRDOWN),
            0x44 => Some(CLKEXT),
            0x48 => Some(CLKINT),
            0x61 => Some(CLKSEL),
            0x68 => Some(RST_PULSE),
            0x70 => Some(PINDRIVE),
            0x71 => Some(PIN_PD_STATE),
            _ => None, // Unknown command
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}
