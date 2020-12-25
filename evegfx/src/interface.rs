use core::cmp::PartialOrd;
use core::convert::TryFrom;

/// Implementations of `EVEInterface` serve as adapters between the interface
/// this library expects and a specific physical implementation of that
/// interface, such as a SPI bus.
///
/// The main library contains no implementations of this trait, in order to
/// make the library portable across systems big and small. Other crates,
/// including some with the name prefix `evegfx`, take on additional
/// dependencies in order to bind this library to specific systems/hardware.
pub trait EVEInterface {
    type Error;

    fn begin_write(&mut self, addr: EVEAddress) -> Result<(), Self::Error>;
    fn begin_read(&mut self, addr: EVEAddress) -> Result<(), Self::Error>;
    fn continue_write(&mut self, v: &[u8]) -> Result<(), Self::Error>;
    fn continue_read(&mut self, into: &mut [u8]) -> Result<(), Self::Error>;
    fn end_write(&mut self) -> Result<(), Self::Error>;
    fn end_read(&mut self) -> Result<(), Self::Error>;
    fn cmd(&mut self, cmd: EVECommand, a0: u8, a1: u8) -> Result<(), Self::Error>;

    fn write(&mut self, addr: EVEAddress, v: &[u8]) -> Result<(), Self::Error> {
        self.begin_write(addr)?;
        self.continue_write(v)?;
        self.end_write()
    }

    fn read(&mut self, addr: EVEAddress, into: &mut [u8]) -> Result<(), Self::Error> {
        self.begin_read(addr)?;
        self.continue_read(into)?;
        self.end_read()
    }
}

/// `EVEAddress` represents a memory address in the memory map of an
/// EVE controller chip.
///
/// An `EVEAddress` value is guaranteed to always be in the valid address
/// range for EVE controllers, which is a 22-bit address space and thus
/// the remaining high-order bits will always be zero.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct EVEAddress(u32);

impl EVEAddress {
    // Mask representing the bits of a u32 that contribute to an EVEAddress.
    pub const MASK: u32 = 0x003fffff;

    /// Check whether the given raw address is within the expected
    /// range for a memory address, returning `true` only if so.
    pub const fn is_valid(raw: u32) -> bool {
        // Only the lowest 22 bits may be nonzero.
        (raw >> 22) == 0
    }

    /// Turns the given raw address value into a valid EVEAddress by masking
    /// out the bits that must always be zero for a valid address.
    ///
    /// This is intended primarily for initializing global constants
    /// representing well-known addresses in the memory map. If you're working
    /// with a dynamically-derived address value then better to use the
    /// `TryInto<u32>` implementation to get an error if the value is out of
    /// range.
    pub const fn force_raw(raw: u32) -> Self {
        Self(raw & Self::MASK)
    }

    /// Write the three bytes needed to form a "write memory" header
    /// for the address into the given bytes. This is a helper for
    /// physical implementations that need to construct a message
    /// buffer to transmit to the real chip, e.g. via SPI.
    pub fn build_write_header(self, into: &mut [u8; 3]) {
        into[0] = (((self.0 >> 16) & 0b00111111) | 0b10000000) as u8;
        into[1] = (self.0 >> 8) as u8;
        into[2] = (self.0 >> 0) as u8;
    }

    /// Write the four bytes needed to form a "read memory" header
    /// for the address into the given bytes. This is a helper for
    /// physical implementations that need to construct a message
    /// buffer to transmit to the real chip, e.g. via SPI.
    pub fn build_read_header(self, into: &mut [u8; 4]) {
        into[0] = ((self.0 >> 16) & 0b00111111) as u8;
        into[1] = (self.0 >> 8) as u8;
        into[2] = (self.0 >> 0) as u8;
        into[3] = 0; // "dummy byte", per the datasheet
    }
}

impl core::fmt::Debug for EVEAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match EVEAddressRegion::containing_offset(*self) {
            Some((region, offset)) => {
                // If it's part of a known region then we'll show it as an
                // offset from that region's base, because that avoids
                // the need to memorize the memory map in order to
                // understand what the address is pointing at.
                write!(f, "({:?} + {:#x})", region, offset)
            }
            None => {
                // Only 22 bits are meaningful, but since we're using hex
                // here we'll show 24 bits worth of hex digits.
                write!(f, "EVEAddress({:#08x})", self.0)
            }
        }
    }
}

/// `EVEAddress` can be converted from a `u32` as long as the value is
/// within the 22-bit address space.
impl TryFrom<u32> for EVEAddress {
    type Error = ();

    fn try_from(raw: u32) -> Result<Self, Self::Error> {
        if Self::is_valid(raw) {
            Ok(Self(raw))
        } else {
            Err(())
        }
    }
}

/// Arithmetic with EVEAddress is 22-bit modular arithmetic, thus ensuring
/// that the result is still always in the expected address range.
impl core::ops::Add<u32> for EVEAddress {
    type Output = Self;

    fn add(self, offset: u32) -> Self {
        Self::force_raw(self.0 + offset)
    }
}

impl core::ops::AddAssign<u32> for EVEAddress {
    fn add_assign(&mut self, offset: u32) {
        self.0 += offset;
        self.0 &= Self::MASK;
    }
}

/// Arithmetic with EVEAddress is 22-bit modular arithmetic, thus ensuring
/// that the result is still always in the expected address range.
impl core::ops::Sub<u32> for EVEAddress {
    type Output = Self;

    fn sub(self, offset: u32) -> Self {
        Self::force_raw(self.0 - offset)
    }
}

impl core::ops::SubAssign<u32> for EVEAddress {
    fn sub_assign(&mut self, offset: u32) {
        self.0 -= offset;
        self.0 &= Self::MASK;
    }
}

/// `EVEAddress` can convert to a u32 whose bits 22 through 31 will always
/// be zero.
impl From<EVEAddress> for u32 {
    fn from(addr: EVEAddress) -> u32 {
        addr.0
    }
}

/// `EVEAddressRegion` represents a region in the EVE memory map.
///
/// While in principle an `EVEAddressRegion` value can represent any consecutive
/// sequence of bytes in the memory space, the `EVEAddressRegion` values
/// defined by this module all match physical regions in the system's memory
/// map, as defined in the EVE datasheets.
///
/// Some EVE devices support an additional address range for external flash
/// memory containing assets. That address space is not covered by `EVEAddress`
/// and thus also not covered by an `EVEAddressRange`. It's used only as the
/// source for static data such as bitmaps and audio.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct EVEAddressRegion {
    pub base: EVEAddress,
    pub length: u32,
}

impl EVEAddressRegion {
    pub const RAM_G: Self = Self {
        base: EVEAddress::force_raw(0x000000),
        length: 1024 << 10,
    };
    pub const ROM: Self = Self {
        base: EVEAddress::force_raw(0x200000),
        length: 1024 << 10,
    };
    pub const RAM_DL: Self = Self {
        base: EVEAddress::force_raw(0x300000),
        length: 8 << 10,
    };
    pub const RAM_REG: Self = Self {
        base: EVEAddress::force_raw(0x302000),
        length: 4 << 10,
    };
    pub const RAM_CMD: Self = Self {
        base: EVEAddress::force_raw(0x308000),
        length: 4 << 10,
    };

    pub const fn contains(&self, addr: EVEAddress) -> bool {
        addr.0 >= self.base.0 && addr.0 < (self.base.0 + self.length)
    }

    /// Returns the address of the given byte offset into the region, with
    /// wrap-around within the region boundary if the offset is greater than
    /// the region's length.
    pub const fn offset(&self, offset: u32) -> EVEAddress {
        let offset = offset % self.length;
        EVEAddress::force_raw(self.base.0 + offset)
    }

    /// Returns the datasheet-defined region that contains the given address,
    /// if any. Returns `None` if the address is not in one of the defined
    /// ranges.
    ///
    /// This is primarily for debug purposes, and is not optimized for use in
    /// normal code. To determine if an address belongs to a specific single
    /// region, call `contains` on that region instead.
    pub const fn containing(addr: EVEAddress) -> Option<Self> {
        if Self::RAM_G.contains(addr) {
            Some(Self::RAM_G)
        } else if Self::RAM_DL.contains(addr) {
            Some(Self::RAM_DL)
        } else if Self::RAM_CMD.contains(addr) {
            Some(Self::RAM_CMD)
        } else if Self::RAM_REG.contains(addr) {
            Some(Self::RAM_REG)
        } else if Self::ROM.contains(addr) {
            Some(Self::ROM)
        } else {
            None
        }
    }

    /// Like `containing` but additionally returns the offset of the address
    /// within the given region, if any. Adding the returned offset to the
    /// returned region will recalculate the original address.
    pub const fn containing_offset(addr: EVEAddress) -> Option<(Self, u32)> {
        match Self::containing(addr) {
            None => None,
            Some(region) => Some((region, addr.0 - region.base.0)),
        }
    }
}

impl core::fmt::Debug for EVEAddressRegion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::RAM_G => {
                write!(f, "EVEAddressRegion::RAM_G")
            }
            Self::RAM_DL => {
                write!(f, "EVEAddressRegion::RAM_DL")
            }
            Self::RAM_CMD => {
                write!(f, "EVEAddressRegion::RAM_CMD")
            }
            Self::RAM_REG => {
                write!(f, "EVEAddressRegion::RAM_REG")
            }
            Self::ROM => {
                write!(f, "EVEAddressRegion::ROM")
            }
            _ => f
                .debug_struct("EVEAddressRegion")
                .field("base", &self.base)
                .field("length", &self.length)
                .finish(),
        }
    }
}

impl core::ops::Add<u32> for EVEAddressRegion {
    type Output = EVEAddress;

    fn add(self, offset: u32) -> EVEAddress {
        self.offset(offset)
    }
}

/// `EVECommand` represents a command for an EVE controller chip.
///
/// An `EVECommand` is guaranteed to always be within the valid range of
/// values for commands, although it may not necessarily match a particular
/// valid command for the target chip.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct EVECommand(u8);

impl EVECommand {
    /// Check whether the given raw address is within the expected
    /// range for a command address, returning `true` only if so.
    pub fn is_valid(raw: u8) -> bool {
        // The two high-order bits must always be 0b01 in a command. Otherwise
        // it would be understood as either a write or read memory address.
        raw == 0 || (raw & 0b11000000) == 0b01000000
    }

    /// Turns the given raw value into a valid EVECommand by masking out the
    /// bits that are expected to be constant for all commands.
    ///
    /// This is intended primarily for initializing global constants
    /// representing well-known commands from an EVE datasheet. If you pass
    /// a value that isn't from a datasheet then the result is likely to
    /// be garbage.
    pub const fn force_raw(raw: u8) -> Self {
        if raw == 0 {
            // The ACTIVE command is special in that it's encoded as all zeros,
            // even though that doesn't match the normal pattern of commands
            // always having 0b01 in their MSBs.
            // (This is the same encoding as reading zero bytes from memory
            // address zero.)
            return Self(0);
        }
        Self((raw & 0b00111111) | 0b01000000)
    }

    pub const fn raw(&self) -> u8 {
        self.0
    }

    /// Write the three bytes needed to form a command message into the given
    /// bytes. This is a helper for physical implementations that need to
    /// construct a message buffer to transmit to the real chip, e.g. via SPI.
    ///
    /// You must provide suitable values for the first and second argument
    /// bytes. These will be written verbatim with no validation into the
    /// appropriate positions in the message.
    ///
    /// In all EVE implementations up to the time of writing, the second
    /// argument must always be zero. It is exposed here only for
    /// forward-compatibility in case any future extensions will use it.
    pub fn build_message(self, a0: u8, a1: u8, into: &mut [u8; 3]) {
        into[0] = self.0;
        into[1] = a0;
        into[2] = a1;
    }
}

impl core::fmt::Debug for EVECommand {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "EVECommand({:#04x})", self.0)
    }
}

/// `EVECommand` can be converted from a `u8` as long as the value is
/// within the 22-bit address space.
impl TryFrom<u8> for EVECommand {
    type Error = ();

    fn try_from(raw: u8) -> Result<Self, Self::Error> {
        if Self::is_valid(raw) {
            Ok(Self(raw))
        } else {
            Err(())
        }
    }
}

/// `EVECommand` can convert to a u8 representing the raw command number.
impl From<EVECommand> for u8 {
    fn from(addr: EVECommand) -> u8 {
        addr.0
    }
}

/// Read the raw chip ID data from the given interface. This is a helper
/// for callers of the lowest-level interface API. Higher layers may
/// provide a more abstract form of this helper which interpret the raw
/// ID data in some way.
///
/// The chip ID data is in the general RAM space, so you should read it
/// only during early startup, before an application has potentially written
/// other data over the top of it.
pub fn read_chip_id<I: EVEInterface>(ei: &mut I) -> Result<[u8; 4], I::Error> {
    let mut into: [u8; 4] = [0; 4];
    ei.read(EVEAddress::force_raw(0xC0000), &mut into)?;
    Ok(into)
}

// We use std in test mode only, so we can do dynamic allocation in the mock
// code.
#[cfg(test)]
pub mod testing {
    extern crate std;

    use super::{EVEAddress, EVECommand, EVEInterface};
    use std::collections::HashMap;
    use std::vec::Vec;

    /// A test double for `trait Interface`, available only in test mode.
    pub struct MockInterface {
        _write_addr: Option<EVEAddress>,
        _read_addr: Option<EVEAddress>,

        // _mem is a sparse representation of the memory space which
        // remembers what was written into it and returns 0xff if asked
        // for an address that wasn't previously written.
        _mem: HashMap<EVEAddress, u8>,

        // if _fail is Some then the mock methods will call it and use the
        // result to decide whether to return an error.
        _fail: Option<fn(&MockInterfaceCall) -> bool>,

        // _calls is the call log. Each call to a mock method appends one
        // entry to this vector, including any that fail.
        _calls: Vec<MockInterfaceCall>,
    }

    #[derive(Clone, Debug)]
    pub enum MockInterfaceCall {
        BeginWrite(EVEAddress),
        ContinueWrite(Vec<u8>),
        EndWrite(EVEAddress),
        BeginRead(EVEAddress),
        ContinueRead(usize),
        EndRead(EVEAddress),
        Cmd(EVECommand, u8, u8),
    }

    impl MockInterface {
        const DEFAULT_MEM_VALUE: u8 = 0xff;

        pub fn new() -> Self {
            Self {
                _write_addr: None,
                _read_addr: None,
                _mem: HashMap::new(),
                _fail: None,
                _calls: Vec::new(),
            }
        }

        /// Consumes the mock and returns all of the calls it logged
        /// during its life.
        pub fn calls(self) -> Vec<MockInterfaceCall> {
            self._calls
        }

        // Copies some data into the fake memory without considering it
        // to be a logged operation. This is intended for setting up
        // memory ready for subsequent calls to `read`.
        pub fn setup_mem(&mut self, addr: EVEAddress, buf: &[u8]) {
            for (i, v) in buf.iter().enumerate() {
                let e_addr = addr + (i as u32);
                let v = *v; // Take a copy of the value from the input
                if v == Self::DEFAULT_MEM_VALUE {
                    self._mem.remove(&e_addr);
                } else {
                    self._mem.insert(e_addr, v);
                }
            }
        }

        fn call_should_fail(&self, call: &MockInterfaceCall) -> bool {
            if let Some(fail) = self._fail {
                if fail(&call) {
                    return true;
                }
            }
            return false;
        }
    }

    impl EVEInterface for MockInterface {
        type Error = ();

        fn begin_write(&mut self, addr: EVEAddress) -> core::result::Result<(), ()> {
            let call = MockInterfaceCall::BeginWrite(addr);
            if self.call_should_fail(&call) {
                self._calls.push(call);
                return Err(());
            }
            self._calls.push(call);
            self._write_addr = Some(addr);
            Ok(())
        }

        fn continue_write(&mut self, buf: &[u8]) -> core::result::Result<(), ()> {
            let log_buf = buf.to_vec();
            let call = MockInterfaceCall::ContinueWrite(log_buf);
            if self.call_should_fail(&call) {
                self._calls.push(call);
                return Err(());
            }
            self._calls.push(call);

            let addr = self._write_addr.unwrap();
            self.setup_mem(addr, buf);
            Ok(())
        }

        fn end_write(&mut self) -> core::result::Result<(), ()> {
            let addr = self._write_addr.unwrap();
            let call = MockInterfaceCall::EndWrite(addr);
            if self.call_should_fail(&call) {
                self._calls.push(call);
                return Err(());
            }
            self._calls.push(call);
            self._write_addr = None;
            Ok(())
        }

        fn begin_read(&mut self, addr: EVEAddress) -> core::result::Result<(), ()> {
            let call = MockInterfaceCall::BeginRead(addr);
            if self.call_should_fail(&call) {
                self._calls.push(call);
                return Err(());
            }
            self._calls.push(call);
            self._read_addr = Some(addr);
            Ok(())
        }

        fn continue_read(&mut self, into: &mut [u8]) -> core::result::Result<(), ()> {
            let call = MockInterfaceCall::ContinueRead(into.len());
            if self.call_should_fail(&call) {
                self._calls.push(call);
                return Err(());
            }
            self._calls.push(call);

            let addr = self._read_addr.unwrap();
            for i in 0..into.len() {
                let e_addr = addr + (i as u32);
                let v = self
                    ._mem
                    .get(&e_addr)
                    .unwrap_or(&Self::DEFAULT_MEM_VALUE)
                    .clone();
                into[i] = v;
            }
            Ok(())
        }

        fn end_read(&mut self) -> core::result::Result<(), ()> {
            let addr = self._read_addr.unwrap();
            let call = MockInterfaceCall::EndRead(addr);
            if self.call_should_fail(&call) {
                self._calls.push(call);
                return Err(());
            }
            self._calls.push(call);
            self._read_addr = None;
            Ok(())
        }

        fn cmd(&mut self, cmd: EVECommand, a0: u8, a1: u8) -> core::result::Result<(), ()> {
            let call = MockInterfaceCall::Cmd(cmd, a0, a1);
            if let Some(fail) = self._fail {
                if fail(&call) {
                    self._calls.push(call);
                    return Err(());
                }
            }
            self._calls.push(call);
            Ok(())
        }
    }

    impl PartialEq for MockInterfaceCall {
        fn eq(&self, other: &Self) -> bool {
            match self {
                MockInterfaceCall::BeginWrite(self_addr) => {
                    if let MockInterfaceCall::BeginWrite(other_addr) = other {
                        *self_addr == *other_addr
                    } else {
                        false
                    }
                }
                MockInterfaceCall::ContinueWrite(self_data) => {
                    if let MockInterfaceCall::ContinueWrite(other_data) = other {
                        self_data.eq(other_data)
                    } else {
                        false
                    }
                }
                MockInterfaceCall::EndWrite(self_addr) => {
                    if let MockInterfaceCall::EndWrite(other_addr) = other {
                        *self_addr == *other_addr
                    } else {
                        false
                    }
                }
                MockInterfaceCall::BeginRead(self_addr) => {
                    if let MockInterfaceCall::BeginRead(other_addr) = other {
                        *self_addr == *other_addr
                    } else {
                        false
                    }
                }
                MockInterfaceCall::ContinueRead(self_len) => {
                    if let MockInterfaceCall::ContinueRead(other_len) = other {
                        *self_len == *other_len
                    } else {
                        false
                    }
                }
                MockInterfaceCall::EndRead(self_addr) => {
                    if let MockInterfaceCall::EndRead(other_addr) = other {
                        *self_addr == *other_addr
                    } else {
                        false
                    }
                }
                MockInterfaceCall::Cmd(self_cmd, self_a0, self_a1) => {
                    if let MockInterfaceCall::Cmd(other_cmd, other_a0, other_a1) = other {
                        *self_cmd == *other_cmd && *self_a0 == *other_a0 && *self_a1 == *other_a1
                    } else {
                        false
                    }
                }
            }
        }
    }
}
