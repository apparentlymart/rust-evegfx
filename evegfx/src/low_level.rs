//! Control of an EVE chip at the level of directly accessing its memory-mapped
//! peripherals.

pub(crate) mod host_commands;
pub(crate) mod registers;

use crate::display_list::DLCmd;
use crate::interface::Interface;
use crate::memory::{HostAccessible, MemoryRegion, Ptr};
use crate::models::Model;
use core::marker::PhantomData;

#[doc(inline)]
pub use registers::Register;

#[doc(inline)]
pub use host_commands::HostCmd;

/// `LowLevel` is a low-level interface to EVE controllers which matches
/// the primitive operations used in Programmers Guides for the various
/// EVE controllers.
///
/// This is slightly higher-level than the `Interface` trait, providing
/// size-specific memory accesses, but doesn't have any special knowledge
/// about the memory map or command set of any particular EVE implementation.
///
/// This struct tracks a "cursor" for appending display list entries using
/// the `dl` method. Use `dl_reset` to reset that cursor to the beginning of
/// display list memory, which you'll typically (but not necessarily) do after
/// writing `REG_DLSWAP` to swap the display list double buffer. `LowLevel`
/// doesn't manage display list buffer swapping itself, only the cursor for
/// the next `dl` call.
pub struct LowLevel<M: Model, I: Interface> {
    raw: I,
    next_dl: Ptr<M::DisplayListMem>,
    _model: PhantomData<M>,
}

impl<M: Model, I: Interface> LowLevel<M, I> {
    const END_ADDR: u32 = M::DisplayListMem::BASE_ADDR + M::DisplayListMem::LENGTH;

    pub fn new(interface: I) -> Self {
        LowLevel {
            raw: interface,
            next_dl: M::DisplayListMem::ptr(0),
            _model: PhantomData,
        }
    }

    /// Consumes the `LowLevel` object and returns the interface it was
    /// originally created with.
    pub fn take_interface(self) -> I {
        self.raw
    }

    pub fn borrow_interface<'a>(&'a mut self) -> &'a mut I {
        &mut self.raw
    }

    pub fn wr8<R: HostAccessible>(&mut self, addr: Ptr<R>, v: u8) -> Result<(), I::Error> {
        let data: [u8; 1] = [v];
        self.raw.write(addr.to_raw(), &data)
    }

    pub fn wr16<R: HostAccessible>(&mut self, addr: Ptr<R>, v: u16) -> Result<(), I::Error> {
        let data: [u8; 2] = [v as u8, (v >> 8) as u8];
        self.raw.write(addr.to_raw(), &data)
    }

    pub fn wr32<R: HostAccessible>(&mut self, addr: Ptr<R>, v: u32) -> Result<(), I::Error> {
        //let data: [u8; 4] = [(v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8];
        let data: [u8; 4] = [v as u8, (v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8];
        self.raw.write(addr.to_raw(), &data)
    }

    pub fn wr8s<R: HostAccessible>(&mut self, addr: Ptr<R>, v: &[u8]) -> Result<(), I::Error> {
        self.raw.write(addr.to_raw(), v)
    }

    pub fn rd8<R: HostAccessible>(&mut self, addr: Ptr<R>) -> Result<u8, I::Error> {
        let mut data: [u8; 1] = [0; 1];
        self.raw.read(addr.to_raw(), &mut data)?;
        Ok(data[0])
    }

    pub fn rd16<R: HostAccessible>(&mut self, addr: Ptr<R>) -> Result<u16, I::Error> {
        let mut data: [u8; 2] = [0; 2];
        self.raw.read(addr.to_raw(), &mut data)?;
        Ok((data[0] as u16) | (data[1] as u16) << 8)
    }

    pub fn rd32<R: HostAccessible>(&mut self, addr: Ptr<R>) -> Result<u32, I::Error> {
        let mut data: [u8; 4] = [0; 4];
        self.raw.read(addr.to_raw(), &mut data)?;
        Ok((data[0] as u32)
            | (data[1] as u32) << 8
            | (data[2] as u32) << 16
            | (data[3] as u32) << 24)
    }

    pub fn rd8s<R: HostAccessible>(
        &mut self,
        addr: Ptr<R>,
        into: &mut [u8],
    ) -> Result<(), I::Error> {
        self.raw.read(addr.to_raw(), into)
    }

    pub fn main_mem_ptr(&self, offset: u32) -> Ptr<M::MainMem> {
        M::MainMem::ptr(offset)
    }

    pub fn reg_ptr(&self, reg: crate::registers::Register) -> Ptr<M::RegisterMem> {
        reg.ptr::<M>()
    }

    pub fn host_command(&mut self, cmd: HostCmd, a0: u8, a1: u8) -> Result<(), I::Error> {
        self.raw.host_cmd(cmd.to_raw(), a0, a1)
    }

    pub fn dl_reset(&mut self) {
        self.next_dl = M::DisplayListMem::ptr(0);
    }

    pub fn dl(&mut self, cmd: DLCmd) -> Result<(), I::Error> {
        self.wr32(self.next_dl, cmd.into())?;

        let mut next_addr = self.next_dl.to_raw() + DLCmd::LENGTH;
        // If the next write would be out of range then we'll undo
        // our increment and let the next write just overwrite the
        // final entry in the display list. This means that a well-behaved
        // program which finishes its display list with the Display
        // command will still end up with that command at its end, even
        // though some of the commands were lost due to running out of
        // display list memory.
        if next_addr >= Self::END_ADDR {
            next_addr = Self::END_ADDR - DLCmd::LENGTH;
        }

        self.next_dl = M::DisplayListMem::ptr(next_addr);
        Ok(())
    }
}

impl<M: Model, I: Interface> crate::display_list::Builder for LowLevel<M, I> {
    type Error = I::Error;

    fn append_raw_command(&mut self, raw: u32) -> core::result::Result<(), I::Error> {
        self.dl(DLCmd::from_raw(raw))
    }

    fn append_command(&mut self, cmd: DLCmd) -> core::result::Result<(), I::Error> {
        self.dl(cmd)
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::interface::testing::{MockInterface, MockInterfaceCall};
    use crate::models::testing::Exhaustive;
    use std::vec;

    fn test_obj<F: FnOnce(&mut MockInterface)>(setup: F) -> LowLevel<Exhaustive, MockInterface> {
        let mut interface = MockInterface::new();
        setup(&mut interface);
        LowLevel::new(interface)
    }

    #[test]
    fn test_wr8() {
        let mut eve = test_obj(|_| {});
        let addr = eve.main_mem_ptr(0x1f);
        eve.wr8(addr, 0x34).unwrap();
        let got = eve.take_interface().calls();
        let want = vec![
            MockInterfaceCall::BeginWrite(0x1f),
            MockInterfaceCall::ContinueWrite(vec![0x34 as u8]),
            MockInterfaceCall::EndWrite(0x1f),
        ];
        assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_wr16() {
        let mut eve = test_obj(|_| {});
        let addr = eve.main_mem_ptr(0x2f);
        eve.wr16(addr, 0x1234).unwrap();
        let got = eve.take_interface().calls();
        let want = vec![
            MockInterfaceCall::BeginWrite(0x2f),
            MockInterfaceCall::ContinueWrite(vec![0x34, 0x12 as u8]),
            MockInterfaceCall::EndWrite(0x2f),
        ];
        assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_wr32() {
        let mut eve = test_obj(|_| {});
        let addr = eve.main_mem_ptr(0x3f);
        eve.wr32(addr, 0x12345678).unwrap();
        let got = eve.take_interface().calls();
        let want = vec![
            MockInterfaceCall::BeginWrite(0x3f),
            MockInterfaceCall::ContinueWrite(vec![0x78, 0x56, 0x34, 0x12 as u8]),
            MockInterfaceCall::EndWrite(0x3f),
        ];
        assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_wr8s() {
        let mut eve = test_obj(|_| {});
        let addr = eve.main_mem_ptr(0x3f);
        let data = ['h' as u8, 'e' as u8, 'l' as u8, 'l' as u8, 'o' as u8];
        eve.wr8s(addr, &data[..]).unwrap();
        let got = eve.take_interface().calls();
        let want = vec![
            MockInterfaceCall::BeginWrite(0x3f),
            MockInterfaceCall::ContinueWrite(vec![
                'h' as u8, 'e' as u8, 'l' as u8, 'l' as u8, 'o' as u8,
            ]),
            MockInterfaceCall::EndWrite(0x3f),
        ];
        assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_rd8() {
        let mut eve = test_obj(|ei| {
            let data = [0x34 as u8];
            ei.setup_mem(0x1f, &data[..])
        });
        let addr = eve.main_mem_ptr(0x1f);
        let got = eve.rd8(addr).unwrap();
        assert_eq!(got, 0x34);

        let got_calls = eve.take_interface().calls();
        let want_calls = vec![
            MockInterfaceCall::BeginRead(0x1f),
            MockInterfaceCall::ContinueRead(1),
            MockInterfaceCall::EndRead(0x1f),
        ];
        assert_eq!(&got_calls[..], &want_calls[..]);
    }

    #[test]
    fn test_rd16() {
        let mut eve = test_obj(|ei| {
            let data = [0x12, 0x34 as u8];
            ei.setup_mem(0x2f, &data[..])
        });
        let addr = eve.main_mem_ptr(0x2f);
        let got = eve.rd16(addr).unwrap();
        assert_eq!(got, 0x3412); // EVE is little-endian

        let got_calls = eve.take_interface().calls();
        let want_calls = vec![
            MockInterfaceCall::BeginRead(0x2f),
            MockInterfaceCall::ContinueRead(2),
            MockInterfaceCall::EndRead(0x2f),
        ];
        assert_eq!(&got_calls[..], &want_calls[..]);
    }

    #[test]
    fn test_rd32() {
        let mut eve = test_obj(|ei| {
            let data = [0x12, 0x34, 0x56, 0x78 as u8];
            ei.setup_mem(0x3f, &data[..])
        });
        let addr = eve.main_mem_ptr(0x3f);
        let got = eve.rd32(addr).unwrap();
        assert_eq!(got, 0x78563412); // EVE is little-endian

        let got_calls = eve.take_interface().calls();
        let want_calls = vec![
            MockInterfaceCall::BeginRead(0x3f),
            MockInterfaceCall::ContinueRead(4),
            MockInterfaceCall::EndRead(0x3f),
        ];
        assert_eq!(&got_calls[..], &want_calls[..]);
    }

    #[test]
    fn test_rd8s() {
        let orig_data = b"hello";
        let mut eve = test_obj(|ei| ei.setup_mem(0x4f, &orig_data[..]));
        let addr = eve.main_mem_ptr(0x4f);
        let mut read_data: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
        eve.rd8s(addr, &mut read_data).unwrap();

        // We read more than we wrote, so we'll see some default
        // values here standing in for uninitialized memory.
        let want_data = b"hello\xff\xff\xff";
        assert_eq!(&read_data[..], &want_data[..]);

        let got_calls = eve.take_interface().calls();
        let want_calls = vec![
            MockInterfaceCall::BeginRead(0x4f),
            MockInterfaceCall::ContinueRead(8),
            MockInterfaceCall::EndRead(0x4f),
        ];
        assert_eq!(&got_calls[..], &want_calls[..]);
    }

    #[test]
    fn test_host_command() {
        let mut eve = test_obj(|_| {});
        eve.host_command(HostCmd::ACTIVE, 0x23, 0x45).unwrap();

        let got_calls = eve.take_interface().calls();
        let want_calls = vec![MockInterfaceCall::Cmd(0x00, 0x23, 0x45)];
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

        let base = <Exhaustive as Model>::DisplayListMem::BASE_ADDR;

        let got_calls = eve.take_interface().calls();
        let want_calls = vec![
            MockInterfaceCall::BeginWrite(base + 0),
            MockInterfaceCall::ContinueWrite(vec![2, 0, 0, 31 as u8]),
            MockInterfaceCall::EndWrite(base + 0),
            MockInterfaceCall::BeginWrite(base + 4),
            MockInterfaceCall::ContinueWrite(vec![3, 0, 0, 9 as u8]),
            MockInterfaceCall::EndWrite(base + 4),
            MockInterfaceCall::BeginWrite(base + 0),
            MockInterfaceCall::ContinueWrite(vec![1, 0, 0, 31 as u8]),
            MockInterfaceCall::EndWrite(base + 0),
        ];
        assert_eq!(&got_calls[..], &want_calls[..]);
    }
}
