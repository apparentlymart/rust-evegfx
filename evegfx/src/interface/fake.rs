//! Fake `Interface` implementation for testing and examples.

use crate::commands::waiter::PollingWaiter;
use crate::commands::Coprocessor;
use crate::interface;
use crate::low_level::Register;
use crate::memory::MemoryRegion;
use crate::models::fake::Model as FakeModel;
use crate::models::Model;

pub type WithFakeModel<'a> = Interface<'a, FakeModel>;

/// A particular set of `Interface` parameters used with the
/// `interface_example` function.
#[doc(hidden)]
type ExampleInterface<'a> = Interface<'a, FakeModel, &'a mut [u32]>;

/// A particular set of `Interface` parameters used with the
/// `coprocessor_example` function.
#[doc(hidden)]
type ExampleCoprocessor<'a> =
    Coprocessor<FakeModel, ExampleInterface<'a>, PollingWaiter<FakeModel, ExampleInterface<'a>>>;

/// Run the given function with a ready-to-use instance of the fake
/// `Interface`.
///
/// This is only here to make it easy to write testable code examples in the
/// crate documentation.
#[doc(hidden)]
pub fn interface_example(f: impl FnOnce(ExampleInterface)) {
    const KB: usize = 1024;
    let mut mem: [u8; 1024 * KB] = [0; 1024 * KB];
    let mut regs: [u32; 1 * KB] = [0; 1 * KB];
    let mut dl: [u8; 8 * KB] = [0; 8 * KB];
    let mut cmd: [u8; 4 * KB] = [0; 4 * KB];

    // We'll set up a few registers with predefined values that other
    // code tends to rely on.
    regs[Register::CMDB_SPACE.index()] = 4092;

    let ei = Interface::new(FakeModel)
        .with_main_ram(&mut mem[..])
        .with_display_list_ram(&mut dl[..])
        .with_cmd_ram(&mut cmd[..])
        .with_register_file(&mut regs[..]);
    f(ei);
}

/// Run the given function with a ready-to-use instance of `Coprocessor`
/// associated with the fake `Interface`.
///
/// This is only here to make it easy to write testable code examples in the
/// crate documentation.
#[doc(hidden)]
pub fn coprocessor_example(f: impl FnOnce(ExampleCoprocessor)) {
    interface_example(|ei| {
        let cp = Coprocessor::new_polling(ei).unwrap();
        f(cp);
    })
}

/// An implementation of [`Interface`](super::Interface) which just reads and
/// writes a buffer in local RAM.
///
/// This type alone doesn't implement any of the typical functionality that
/// would be expected from an EVE chip, but it could in principle be used in
/// conjunction with other code (probably running in a separate thread) to
/// simulate parts of the EVE functionality for testing purposes.
///
/// This is mainly here just so there's a simple backend to write tests and
/// examples against.
pub struct Interface<'a, M: Model, RF: RegisterFile = NoRegisterFile> {
    main_ram: &'a mut [u8],
    display_list_ram: &'a mut [u8],
    registers: RF,
    cmd_ram: &'a mut [u8],

    write_addr: Option<u32>,
    read_addr: Option<u32>,

    _model: core::marker::PhantomData<M>,
}

impl<'a, M: Model> Interface<'a, M, NoRegisterFile> {
    pub fn new(_model: M) -> Self {
        Self {
            main_ram: &mut [],
            display_list_ram: &mut [],
            registers: NoRegisterFile,
            cmd_ram: &mut [],

            write_addr: None,
            read_addr: None,

            _model: core::marker::PhantomData,
        }
    }
}

impl<'a, M: Model, RF: RegisterFile> Interface<'a, M, RF> {
    pub fn with_main_ram(self, buf: &'a mut [u8]) -> Self {
        Self {
            main_ram: buf,
            display_list_ram: self.display_list_ram,
            registers: self.registers,
            cmd_ram: self.cmd_ram,
            write_addr: self.write_addr,
            read_addr: self.read_addr,
            _model: self._model,
        }
    }
    pub fn with_display_list_ram(self, buf: &'a mut [u8]) -> Self {
        Self {
            main_ram: self.main_ram,
            display_list_ram: buf,
            registers: self.registers,
            cmd_ram: self.cmd_ram,
            write_addr: self.write_addr,
            read_addr: self.read_addr,
            _model: self._model,
        }
    }
    pub fn with_register_file<RF2: RegisterFile>(self, new: RF2) -> Interface<'a, M, RF2> {
        Interface {
            main_ram: self.main_ram,
            display_list_ram: self.display_list_ram,
            registers: new,
            cmd_ram: self.cmd_ram,
            write_addr: self.write_addr,
            read_addr: self.read_addr,
            _model: self._model,
        }
    }
    pub fn with_cmd_ram(self, buf: &'a mut [u8]) -> Self {
        Self {
            main_ram: self.main_ram,
            display_list_ram: self.display_list_ram,
            registers: self.registers,
            cmd_ram: buf,
            write_addr: self.write_addr,
            read_addr: self.read_addr,
            _model: self._model,
        }
    }

    pub fn model_main_mem_size() -> u32 {
        M::MainMem::LENGTH
    }

    pub fn model_display_list_mem_size() -> u32 {
        M::DisplayListMem::LENGTH
    }

    pub fn model_register_count() -> u32 {
        M::RegisterMem::LENGTH / 4
    }

    pub fn model_command_mem_size() -> u32 {
        M::CommandMem::LENGTH
    }

    fn offset_addr(addr: u32) -> OffsetAddr {
        if M::MainMem::contains_addr(addr) {
            return OffsetAddr::Main(addr - M::MainMem::BASE_ADDR);
        }
        if M::DisplayListMem::contains_addr(addr) {
            return OffsetAddr::DisplayList(addr - M::DisplayListMem::BASE_ADDR);
        }
        if M::RegisterMem::contains_addr(addr) {
            return OffsetAddr::Registers(addr - M::RegisterMem::BASE_ADDR);
        }
        if M::CommandMem::contains_addr(addr) {
            return OffsetAddr::Command(addr - M::CommandMem::BASE_ADDR);
        }
        OffsetAddr::Unknown
    }

    fn main_mem_result<R>(
        r: Result<R, SliceError>,
    ) -> Result<R, <Self as interface::Interface>::Error> {
        match r {
            Ok(v) => Ok(v),
            Err(err) => Err(Error::MainMem(err)),
        }
    }

    fn dl_mem_result<R>(
        r: Result<R, SliceError>,
    ) -> Result<R, <Self as interface::Interface>::Error> {
        match r {
            Ok(v) => Ok(v),
            Err(err) => Err(Error::DisplayListMem(err)),
        }
    }

    fn reg_result<R>(
        r: Result<R, RegisterError<RF::Error>>,
    ) -> Result<R, <Self as interface::Interface>::Error> {
        match r {
            Ok(v) => Ok(v),
            Err(err) => Err(Error::Registers(err)),
        }
    }

    fn cmd_mem_result<R>(
        r: Result<R, SliceError>,
    ) -> Result<R, <Self as interface::Interface>::Error> {
        match r {
            Ok(v) => Ok(v),
            Err(err) => Err(Error::CommandMem(err)),
        }
    }
}

impl<'a, M: Model, RF: RegisterFile> super::Interface for Interface<'a, M, RF> {
    type Error = Error<RF::Error>;

    fn begin_write(&mut self, addr: u32) -> core::result::Result<(), Self::Error> {
        if let Some(_) = self.write_addr {
            return Err(Error::IncorrectSequence);
        }
        if let Some(_) = self.read_addr {
            return Err(Error::IncorrectSequence);
        }
        self.write_addr = Some(addr);
        Ok(())
    }

    fn continue_write(&mut self, data: &[u8]) -> core::result::Result<(), Self::Error> {
        if let Some(addr) = self.write_addr {
            use OffsetAddr::*;
            match Self::offset_addr(addr) {
                Main(offset) => {
                    let new_addr =
                        (<M as Model>::MainMem::ptr(offset) + data.len() as u32).to_raw();
                    self.write_addr = Some(new_addr);
                    Self::main_mem_result(self.main_ram.mm_write(offset, data))
                }
                DisplayList(offset) => {
                    let new_addr =
                        (<M as Model>::DisplayListMem::ptr(offset) + data.len() as u32).to_raw();
                    self.write_addr = Some(new_addr);
                    Self::dl_mem_result(self.display_list_ram.mm_write(offset, data))
                }
                Registers(offset) => Self::reg_result(self.registers.mm_write(offset, data)),
                Command(offset) => {
                    let new_addr =
                        (<M as Model>::CommandMem::ptr(offset) + data.len() as u32).to_raw();
                    self.write_addr = Some(new_addr);
                    Self::cmd_mem_result(self.cmd_ram.mm_write(offset, data))
                }
                Unknown => Err(Error::UnmappedAddr),
            }
        } else {
            Err(Error::IncorrectSequence)
        }
    }

    fn end_write(&mut self) -> core::result::Result<(), Self::Error> {
        if let Some(_) = self.write_addr {
            self.write_addr = None;
            Ok(())
        } else {
            Err(Error::IncorrectSequence)
        }
    }

    fn begin_read(&mut self, addr: u32) -> core::result::Result<(), Self::Error> {
        if let Some(_) = self.write_addr {
            return Err(Error::IncorrectSequence);
        }
        if let Some(_) = self.read_addr {
            return Err(Error::IncorrectSequence);
        }
        self.read_addr = Some(addr);
        Ok(())
    }

    fn continue_read(&mut self, into: &mut [u8]) -> core::result::Result<(), Self::Error> {
        if let Some(addr) = self.read_addr {
            use OffsetAddr::*;
            match Self::offset_addr(addr) {
                Main(offset) => {
                    let new_addr =
                        (<M as Model>::MainMem::ptr(offset) + into.len() as u32).to_raw();
                    self.write_addr = Some(new_addr);
                    Self::main_mem_result(self.main_ram.mm_read(offset, into))
                }
                DisplayList(offset) => {
                    let new_addr =
                        (<M as Model>::DisplayListMem::ptr(offset) + into.len() as u32).to_raw();
                    self.write_addr = Some(new_addr);
                    Self::dl_mem_result(self.display_list_ram.mm_read(offset, into))
                }
                Registers(offset) => Self::reg_result(self.registers.mm_read(offset, into)),
                Command(offset) => {
                    let new_addr =
                        (<M as Model>::CommandMem::ptr(offset) + into.len() as u32).to_raw();
                    self.write_addr = Some(new_addr);
                    Self::cmd_mem_result(self.cmd_ram.mm_read(offset, into))
                }
                Unknown => Err(Error::UnmappedAddr),
            }
        } else {
            Err(Error::IncorrectSequence)
        }
    }

    fn end_read(&mut self) -> core::result::Result<(), Self::Error> {
        if let Some(_) = self.read_addr {
            self.read_addr = None;
            Ok(())
        } else {
            Err(Error::IncorrectSequence)
        }
    }

    fn host_cmd(&mut self, _cmd: u8, _a0: u8, _a1: u8) -> core::result::Result<(), Self::Error> {
        // For now the fake interface doesn't do anything with commands.
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum OffsetAddr {
    Unknown,
    Main(u32),
    DisplayList(u32),
    Registers(u32),
    Command(u32),
}

#[derive(Debug)]
pub enum Error<RegError> {
    IncorrectSequence,
    UnmappedAddr,
    MainMem(SliceError),
    DisplayListMem(SliceError),
    Registers(RegisterError<RegError>),
    CommandMem(SliceError),
}

/// Implemented by types that serve as "hooks" for implementing register
/// behaviors.
pub trait RegisterFile {
    type Error: core::fmt::Debug;

    /// Directly read the backing store for the given register, with no
    /// side-effects and no failures.
    ///
    /// The fake interface uses this to implement partial writes to registers,
    /// by first reading out the existing value and masking the new value
    /// into it.
    fn internal_read(&self, reg: Register) -> u32;

    /// Write a new value to the given register, and take any side-effects that
    /// the write might imply.
    fn write(&mut self, reg: Register, v: u32) -> Result<(), Self::Error>;

    /// Read the value of the given register and also take any side-effects
    /// that the read might imply.
    ///
    /// The default implementation of `read` is just a thin wrapper around
    /// `internal_read`. Implementations can override it to add any additional
    /// side-effects.
    fn read(&mut self, reg: Register) -> Result<u32, Self::Error> {
        Ok(self.internal_read(reg))
    }
}

impl RegisterFile for &mut [u32] {
    type Error = SliceError;

    fn internal_read(&self, reg: Register) -> u32 {
        let idx = reg.index();
        if idx >= self.len() {
            return 0x00000000; // an arbitrary placeholder value
        }
        self[idx]
    }

    fn read(&mut self, reg: Register) -> Result<u32, Self::Error> {
        let idx = reg.index();
        if idx >= self.len() {
            return Err(Self::Error::OutOfBounds {
                size: self.len(),
                index: idx,
            });
        }
        Ok(self.internal_read(reg))
    }

    fn write(&mut self, reg: Register, v: u32) -> Result<(), Self::Error> {
        let idx = reg.index();
        if idx >= self.len() {
            return Err(Self::Error::OutOfBounds {
                size: self.len(),
                index: idx,
            });
        }
        self[idx] = v;
        Ok(())
    }
}

/// Error type for when memory operations are backed by a slice value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceError {
    OutOfBounds { size: usize, index: usize },
    Unaligned,
}

/// An implementation of [`RegisterFile`] that doesn't have any registers at
/// all.
///
/// This is a placeholder for situations that don't need to support direct
/// access to registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoRegisterFile;

impl RegisterFile for NoRegisterFile {
    type Error = ();

    fn internal_read(&self, _reg: Register) -> u32 {
        0x00000000
    }

    fn read(&mut self, _reg: Register) -> Result<u32, Self::Error> {
        Err(())
    }

    fn write(&mut self, _reg: Register, _v: u32) -> Result<(), Self::Error> {
        Err(())
    }
}

trait MemoryMapped {
    type Error: core::fmt::Debug;

    fn mm_read(&mut self, offset: u32, into: &mut [u8]) -> Result<(), Self::Error>;
    fn mm_write(&mut self, offset: u32, data: &[u8]) -> Result<(), Self::Error>;
}

impl MemoryMapped for [u8] {
    type Error = SliceError;

    fn mm_read(&mut self, offset: u32, into: &mut [u8]) -> Result<(), Self::Error> {
        let bound = self.len();
        for (i, v) in into.iter_mut().enumerate() {
            let offset = offset as usize + i;
            if offset >= bound {
                return Err(SliceError::OutOfBounds {
                    size: bound,
                    index: offset,
                });
            }
            *v = self[offset];
        }
        Ok(())
    }

    fn mm_write(&mut self, offset: u32, data: &[u8]) -> Result<(), Self::Error> {
        let bound = self.len();
        for (i, v) in data.iter().enumerate() {
            let offset = offset as usize + i;
            if offset >= bound {
                return Err(SliceError::OutOfBounds {
                    size: bound,
                    index: offset,
                });
            }
            self[offset] = *v;
        }
        Ok(())
    }
}

impl<RF: RegisterFile> MemoryMapped for RF {
    type Error = RegisterError<RF::Error>;

    fn mm_read(&mut self, offset: u32, into: &mut [u8]) -> Result<(), Self::Error> {
        use core::convert::TryFrom;
        let len = into.len();
        if (offset % 4) != 0 {
            return Err(Self::Error::Unaligned);
        }
        if len > 4 {
            return Err(Self::Error::Oversize);
        }
        let reg = match Register::try_from(offset as u16) {
            Ok(v) => v,
            Err(_) => return Err(Self::Error::NotRegister(offset as u16)),
        };
        let val = match self.read(reg) {
            Ok(v) => v,
            Err(err) => return Err(Self::Error::Hook(err)),
        };
        for i in 0..len {
            into[i] = (val >> (i * 8)) as u8;
        }
        Ok(())
    }

    fn mm_write(&mut self, offset: u32, data: &[u8]) -> Result<(), Self::Error> {
        use core::convert::TryFrom;
        let len = data.len();
        if (offset % 4) != 0 {
            return Err(Self::Error::Unaligned);
        }
        if len > 4 {
            return Err(Self::Error::Oversize);
        }
        let reg_num = offset as u16;
        let reg = match Register::try_from(reg_num) {
            Ok(v) => v,
            Err(_) => return Err(Self::Error::NotRegister(reg_num)),
        };
        if len == 4 {
            // Easy case: we can just write it in whole.
            let val = data[0] as u32
                | (data[1] as u32) << 8
                | (data[2] as u32) << 16
                | (data[3] as u32) << 24;
            return match self.write(reg, val) {
                Ok(_) => Ok(()),
                Err(err) => return Err(Self::Error::Hook(err)),
            };
        }

        // Otherwise, we need to mask the new data into the already-stored
        // value in order to simulate a partial write.
        let mut val = self.internal_read(reg);
        for (i, v) in data.iter().enumerate() {
            let shift = (i * 8) as usize;
            let mask = !(0xff << shift);
            val = val & mask;
            val = val | ((*v as u32) << shift);
        }
        match self.write(reg, val) {
            Ok(_) => Ok(()),
            Err(err) => return Err(Self::Error::Hook(err)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterError<RFErr> {
    NotRegister(u16),
    OutOfBounds,
    Unaligned,
    Oversize,
    Hook(RFErr),
}
