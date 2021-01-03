use num_enum::{IntoPrimitive, TryFromPrimitive};

/// Represents a register within the MEM_REG region on an EVE device.
#[derive(TryFromPrimitive, IntoPrimitive, Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u16)]
#[allow(non_camel_case_types)]
pub enum Register {
    ADAPTIVE_FRAMERATE = 0x57c,
    CLOCK = 0x08,
    CMD_DL = 0x100,
    CMD_READ = 0xf8,
    CMD_WRITE = 0xfc,
    CMDB_SPACE = 0x574,
    CMDB_WRITE = 0x578,
    COPRO_PATCH_PTR = 0x7162,
    CPURESET = 0x20,
    CSPREAD = 0x68,
    DITHER = 0x60,
    DLSWAP = 0x54,
    FLASH_STATUS = 0x5f0,
    FLASH_SIZE = 0x7024,
    FRAMES = 0x04,
    FREQUENCY = 0x0c,
    GPIO = 0x94,
    GPIO_DIR = 0x90,
    GPIO_X = 0x9c,
    GPIOX_DIR = 0x98,
    HCYCLE = 0x2c,
    HOFFSET = 0x30,
    HSIZE = 0x34,
    HSYNC0 = 0x38,
    HSYNC1 = 0x3c,
    ID = 0x00,
    INT_EN = 0xac,
    INT_FLAGS = 0xa8,
    INT_MASK = 0xb0,
    MACRO_0 = 0xd8,
    MACRO_1 = 0xdc,
    MEDIAFIFO_READ = 0x7014,
    MEDIAFIFO_WRITE = 0x7018,
    OUTBITS = 0x5c,
    PCLK = 0x70,
    PCLK_POL = 0x6c,
    PLAY = 0x8c,
    PLAY_CONTROL = 0x714e,
    PLAYBACK_FORMAT = 0xc4,
    PLAYBACK_FREQ = 0xc0,
    PLAYBACK_LENGTH = 0xb8,
    PLAYBACK_LOOP = 0xc8,
    PLAYBACK_PAUSE = 0x5ec,
    PLAYBACK_PLAY = 0xcc,
    PLAYBACK_READPTR = 0xbc,
    PLAYBACK_START = 0xb4,
    PWM_DUTY = 0xd4,
    PWM_HZ = 0xd0,
    ROTATE = 0x58,
    SOUND = 0x88,
    SPI_WIDTH = 0x180,
    SWIZZLE = 0x64,
    TAG = 0x7c,
    TAG_X = 0x74,
    TAG_Y = 0x78,
    TRACKER = 0x7000,
    TRACKER_1 = 0x7004,
    TRACKER_2 = 0x7008,
    TRACKER_3 = 0x700C,
    TRACKER_4 = 0x7010,
    VCYCLE = 0x40,
    VOFFSET = 0x44,
    VOL_PB = 0x84,
    VSIZE = 0x48,
    VSYNC0 = 0x4c,
    VSYNC1 = 0x50,
}

impl Register {
    pub fn ptr<M: crate::models::Model>(self) -> crate::memory::Ptr<M::RegisterMem> {
        use crate::memory::MemoryRegion;
        M::RegisterMem::ptr(self as u32)
    }

    /// Returns the offset of the register address within the register memory.
    pub fn offset(self) -> u32 {
        self as u32
    }

    /// Returns the index of the register within the register file, as if the
    /// register file were an array of `u32`.
    pub fn index(self) -> usize {
        self as usize / 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::testing::Exhaustive;

    #[test]
    fn test_ptr() {
        assert_eq!(Register::VSYNC1.ptr::<Exhaustive>().to_raw(), 0x302050);
        assert_eq!(Register::VSYNC1.ptr::<Exhaustive>().to_raw(), 0x302050);
    }
}
