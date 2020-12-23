use crate::display_list::DLCmd;
use crate::interface::{EVEAddress, EVEAddressRegion, EVECommand, EVEInterface};

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
            next_dl: EVEAddressRegion::RAM_DL.base,
        }
    }

    /// Consumes the `EVELowLevel` object and returns the interface it was
    /// originally created with.
    pub fn take_interface(self) -> I {
        self.raw
    }

    pub fn wr8(&mut self, addr: EVEAddress, v: u8) -> Result<(), I::Error> {
        let data: [u8; 1] = [v];
        self.raw.write(addr, &data)
    }

    pub fn wr16(&mut self, addr: EVEAddress, v: u16) -> Result<(), I::Error> {
        let data: [u8; 2] = [v as u8, (v >> 8) as u8];
        self.raw.write(addr, &data)
    }

    pub fn wr32(&mut self, addr: EVEAddress, v: u32) -> Result<(), I::Error> {
        //let data: [u8; 4] = [(v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8];
        let data: [u8; 4] = [v as u8, (v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8];
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
        Ok((data[0] as u16) | (data[1] as u16) << 8)
    }

    pub fn rd32(&mut self, addr: EVEAddress) -> Result<u32, I::Error> {
        let mut data: [u8; 4] = [0; 4];
        self.raw.read(addr, &mut data)?;
        Ok((data[0] as u32)
            | (data[1] as u32) << 8
            | (data[2] as u32) << 16
            | (data[3] as u32) << 24)
    }

    pub fn rd8s(&mut self, addr: EVEAddress, into: &mut [u8]) -> Result<(), I::Error> {
        self.raw.read(addr, into)
    }

    pub fn host_command(&mut self, cmd: EVECommand, a0: u8, a1: u8) -> Result<(), I::Error> {
        self.raw.cmd(cmd, a0, a1)
    }

    pub fn dl_reset(&mut self) {
        self.next_dl = EVEAddressRegion::RAM_DL.base;
    }

    pub fn dl(&mut self, cmd: DLCmd) -> Result<(), I::Error> {
        self.wr32(self.next_dl, cmd.into())?;
        self.next_dl += DLCmd::LENGTH; // ready for the next entry
        if !EVEAddressRegion::RAM_DL.contains(self.next_dl) {
            // If the next write would be out of range then we'll undo
            // our increment and let the next write just overwrite the
            // final entry in the display list. This means that a well-behaved
            // program which finishes its display list with the Display
            // command will still end up with that command at its end, even
            // though some of the commands were lost due to running out of
            // display list memory.
            self.next_dl =
                EVEAddressRegion::RAM_DL + (EVEAddressRegion::RAM_DL.length - DLCmd::LENGTH);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::interface::testing::{MockInterface, MockInterfaceCall};
    use core::convert::TryFrom;
    use std::vec;

    fn test_obj<F: FnOnce(&mut MockInterface)>(setup: F) -> EVELowLevel<MockInterface> {
        let mut interface = MockInterface::new();
        setup(&mut interface);
        EVELowLevel::new(interface)
    }

    #[test]
    fn test_wr8() {
        let mut eve = test_obj(|_| {});
        let addr = EVEAddress::force_raw(0x1f);
        eve.wr8(addr, 0x34).unwrap();
        let got = eve.take_interface().calls();
        let want = vec![MockInterfaceCall::Write(addr, vec![0x34 as u8])];
        assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_wr16() {
        let mut eve = test_obj(|_| {});
        let addr = EVEAddress::force_raw(0x2f);
        eve.wr16(addr, 0x1234).unwrap();
        let got = eve.take_interface().calls();
        let want = vec![MockInterfaceCall::Write(addr, vec![0x34, 0x12 as u8])];
        assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_wr32() {
        let mut eve = test_obj(|_| {});
        let addr = EVEAddress::force_raw(0x3f);
        eve.wr32(addr, 0x12345678).unwrap();
        let got = eve.take_interface().calls();
        let want = vec![MockInterfaceCall::Write(
            addr,
            vec![0x78, 0x56, 0x34, 0x12 as u8],
        )];
        assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_wr8s() {
        let mut eve = test_obj(|_| {});
        let addr = EVEAddress::force_raw(0x3f);
        let data = ['h' as u8, 'e' as u8, 'l' as u8, 'l' as u8, 'o' as u8];
        eve.wr8s(addr, &data[..]).unwrap();
        let got = eve.take_interface().calls();
        let want = vec![MockInterfaceCall::Write(
            addr,
            vec!['h' as u8, 'e' as u8, 'l' as u8, 'l' as u8, 'o' as u8],
        )];
        assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_rd8() {
        let addr = EVEAddress::force_raw(0x1f);
        let mut eve = test_obj(|ei| {
            let data = [0x34 as u8];
            ei.setup_mem(addr, &data[..])
        });
        let got = eve.rd8(addr).unwrap();
        assert_eq!(got, 0x34);

        let got_calls = eve.take_interface().calls();
        let want_calls = vec![MockInterfaceCall::Read(addr, 1)];
        assert_eq!(&got_calls[..], &want_calls[..]);
    }

    #[test]
    fn test_rd16() {
        let addr = EVEAddress::force_raw(0x2f);
        let mut eve = test_obj(|ei| {
            let data = [0x12, 0x34 as u8];
            ei.setup_mem(addr, &data[..])
        });
        let got = eve.rd16(addr).unwrap();
        assert_eq!(got, 0x3412); // EVE is little-endian

        let got_calls = eve.take_interface().calls();
        let want_calls = vec![MockInterfaceCall::Read(addr, 2)];
        assert_eq!(&got_calls[..], &want_calls[..]);
    }

    #[test]
    fn test_rd32() {
        let addr = EVEAddress::force_raw(0x3f);
        let mut eve = test_obj(|ei| {
            let data = [0x12, 0x34, 0x56, 0x78 as u8];
            ei.setup_mem(addr, &data[..])
        });
        let got = eve.rd32(addr).unwrap();
        assert_eq!(got, 0x78563412); // EVE is little-endian

        let got_calls = eve.take_interface().calls();
        let want_calls = vec![MockInterfaceCall::Read(addr, 4)];
        assert_eq!(&got_calls[..], &want_calls[..]);
    }

    #[test]
    fn test_rd8s() {
        let addr = EVEAddress::force_raw(0x4f);
        let orig_data = ['h' as u8, 'e' as u8, 'l' as u8, 'l' as u8, 'o' as u8];
        let mut eve = test_obj(|ei| ei.setup_mem(addr, &orig_data[..]));
        let mut read_data: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
        eve.rd8s(addr, &mut read_data).unwrap();

        let want_data = [
            // We read more than we wrote, so we'll see some default
            // values here standing in for uninitialized memory.
            'h' as u8, 'e' as u8, 'l' as u8, 'l' as u8, 'o' as u8, 0xff, 0xff, 0xff,
        ];
        assert_eq!(&read_data[..], &want_data[..]);

        let got_calls = eve.take_interface().calls();
        let want_calls = vec![MockInterfaceCall::Read(addr, 8)];
        assert_eq!(&got_calls[..], &want_calls[..]);
    }

    #[test]
    fn test_host_command() {
        let mut eve = test_obj(|_| {});
        let cmd = EVECommand::try_from(0x42 as u8).unwrap();
        eve.host_command(cmd, 0x23, 0x45).unwrap();

        let got_calls = eve.take_interface().calls();
        let want_calls = vec![MockInterfaceCall::Cmd(cmd, 0x23, 0x45)];
        assert_eq!(&got_calls[..], &want_calls[..]);
    }

    #[test]
    fn test_dl() {
        let mut eve = test_obj(|_| {});

        eve.dl(DLCmd::begin(crate::display_list::GraphicsPrimitive::Points))
            .unwrap();
        eve.dl(DLCmd::alpha_func(
            crate::display_list::AlphaTestFunc::Never,
            3,
        ))
        .unwrap();
        eve.dl_reset();
        eve.dl(DLCmd::begin(
            crate::display_list::GraphicsPrimitive::Bitmaps,
        ))
        .unwrap();

        let got_calls = eve.take_interface().calls();
        let want_calls = vec![
            MockInterfaceCall::Write(EVEAddressRegion::RAM_DL + 0, vec![2, 0, 0, 31 as u8]),
            MockInterfaceCall::Write(EVEAddressRegion::RAM_DL + 4, vec![3, 0, 0, 9 as u8]),
            MockInterfaceCall::Write(EVEAddressRegion::RAM_DL + 0, vec![1, 0, 0, 31 as u8]),
        ];
        assert_eq!(&got_calls[..], &want_calls[..]);
    }
}
