use crate::interface::{EVEAddress, EVEAddressRegion};

/// Represents a register within the MEM_REG region on an EVE device.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
#[allow(non_camel_case_types)]
pub enum EVERegister {
    ID = 0x00,
    CSPREAD = 0x68,
    HCYCLE = 0x2c,
    HOFFSET = 0x30,
    HSIZE = 0x34,
    HSYNC0 = 0x38,
    HSYNC1 = 0x3c,
    PCLK = 0x70,
    PCLK_POL = 0x6c,
    SWIZZLE = 0x64,
    VCYCLE = 0x40,
    VOFFSET = 0x44,
    VSIZE = 0x48,
    VSYNC0 = 0x4c,
    VSYNC1 = 0x50,
}

impl EVERegister {
    pub const fn address(self) -> EVEAddress {
        EVEAddressRegion::RAM_REG.offset(self as u32)
    }
}

impl core::convert::From<EVERegister> for EVEAddress {
    fn from(cmd: EVERegister) -> Self {
        cmd.address()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_address() {
        assert_eq!(
            EVERegister::VSYNC1.address(),
            EVEAddress::force_raw(0x302050)
        );
        assert_eq!(
            EVEAddress::from(EVERegister::VSYNC1),
            EVEAddress::force_raw(0x302050)
        );
    }
}
