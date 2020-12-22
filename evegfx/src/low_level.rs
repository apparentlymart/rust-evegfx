use crate::display_list::DLCmd;
use crate::interface::{EVEAddress, EVECommand, EVEInterface};

pub const RAM_G: EVEAddress = EVEAddress::force_raw(0x000000);
pub const RAM_G_LEN: u32 = 1024 << 10;
pub const ROM: EVEAddress = EVEAddress::force_raw(0x200000);
pub const ROM_LEN: u32 = 1024 << 10;
pub const RAM_DL: EVEAddress = EVEAddress::force_raw(0x300000);
pub const RAM_DL_LEN: u32 = 8 << 10;
pub const RAM_REG: EVEAddress = EVEAddress::force_raw(0x302000);
pub const RAM_REG_LEN: u32 = 4 << 10;
pub const RAM_CMD: EVEAddress = EVEAddress::force_raw(0x302000);
pub const RAM_CMD_LEN: u32 = 4 << 10;

/// `EVELowLevel` is a low-level interface to EVE controllers which matches
/// the primitive operations used in Programmers Guides for the various
/// EVE controllers.
///
/// This is slightly higher-level than the `EVEInterface` trait, providing
/// size-specific memory accesses, but doesn't have any special knowledge
/// about the memory map or command set of any particular EVE implementation.
///
/// This struct tracks a "cursor" for appending display list entries using
/// the `dl` method. Use `dl_reset` to reset that cursor to the beginning of
/// display list memory, which you'll typically (but not necessarily) do after
/// writing `REG_DLSWAP` to swap the display list double buffer. `EVELowLevel`
/// doesn't manage display list buffer swapping itself, only the cursor for
/// the next `dl` call.
pub struct EVELowLevel<I: EVEInterface> {
    raw: I,
    next_dl: EVEAddress,
}

impl<I: EVEInterface> EVELowLevel<I> {
    pub fn new(interface: I) -> Self {
        Self {
            raw: interface,
            next_dl: RAM_DL,
        }
    }

    pub fn wr8(&mut self, addr: EVEAddress, v: u8) -> Result<(), I::Error> {
        let data: [u8; 1] = [v];
        self.raw.write(addr, &data)
    }

    pub fn wr16(&mut self, addr: EVEAddress, v: u16) -> Result<(), I::Error> {
        let data: [u8; 2] = [(v >> 8) as u8, v as u8];
        self.raw.write(addr, &data)
    }

    pub fn wr32(&mut self, addr: EVEAddress, v: u32) -> Result<(), I::Error> {
        let data: [u8; 4] = [(v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8];
        self.raw.write(addr, &data)
    }

    pub fn wr8s(&mut self, addr: EVEAddress, v: &[u8]) -> Result<(), I::Error> {
        self.raw.write(addr, v)
    }

    pub fn rd8(&mut self, addr: EVEAddress) -> Result<u8, I::Error> {
        let mut data: [u8; 1] = [0; 1];
        self.raw.read(addr, &mut data)?;
        Ok(data[0])
    }

    pub fn rd16(&mut self, addr: EVEAddress) -> Result<u16, I::Error> {
        let mut data: [u8; 2] = [0; 2];
        self.raw.read(addr, &mut data)?;
        Ok((data[0] as u16) << 8 | (data[1] as u16))
    }

    pub fn rd32(&mut self, addr: EVEAddress) -> Result<u32, I::Error> {
        let mut data: [u8; 4] = [0; 4];
        self.raw.read(addr, &mut data)?;
        Ok((data[0] as u32) << 24
            | (data[1] as u32) << 16
            | (data[2] as u32) << 8
            | (data[3] as u32))
    }

    pub fn rd8s(&mut self, addr: EVEAddress, into: &mut [u8]) -> Result<(), I::Error> {
        self.raw.read(addr, into)
    }

    pub fn host_command(&mut self, cmd: EVECommand, a0: u8, a1: u8) -> Result<(), I::Error> {
        self.raw.cmd(cmd, a0, a1)
    }

    pub fn dl_reset(&mut self) {
        self.next_dl = RAM_DL;
    }

    pub fn dl(&mut self, cmd: DLCmd) -> Result<(), I::Error> {
        self.wr32(self.next_dl, cmd.into())?;
        self.next_dl += DLCmd::LENGTH; // ready for the next entry
        if self.next_dl >= (RAM_DL + RAM_DL_LEN) {
            // If the next write would be out of range then we'll undo
            // our increment and let the next write just overwrite the
            // final entry in the display list. This means that a well-behaved
            // program which finishes its display list with the Display
            // command will still end up with that command at its end, even
            // though some of the commands were lost due to running out of
            // display list memory.
            self.next_dl = RAM_DL + (RAM_DL_LEN - DLCmd::LENGTH);
        }
        Ok(())
    }
}
