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

    fn write(&mut self, addr: EVEAddress, v: &[u8]) -> Result<(), Self::Error>;
    fn read(&mut self, addr: EVEAddress, into: &mut [u8]) -> Result<(), Self::Error>;
    fn cmd(&mut self, cmd: EVECommand, a0: u8, a1: u8) -> Result<(), Self::Error>;
}

/// `EVEAddress` represents a memory address in the memory map of an
/// EVE controller chip.
///
/// An `EVEAddress` value is guaranteed to always be in the valid address
/// range for EVE controllers, which is a 22-bit address space and thus
/// the remaining high-order bits will always be zero.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Debug, Hash)]
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
        into[0] = ((self.0 >> 16) & 0b00111111) as u8;
        into[1] = (self.0 >> 8) as u8;
        into[2] = (self.0 >> 0) as u8;
    }

    /// Write the four bytes needed to form a "read memory" header
    /// for the address into the given bytes. This is a helper for
    /// physical implementations that need to construct a message
    /// buffer to transmit to the real chip, e.g. via SPI.
    pub fn build_read_header(self, into: &mut [u8; 4]) {
        into[0] = (((self.0 >> 16) & 0b00111111) | 0b10000000) as u8;
        into[1] = (self.0 >> 8) as u8;
        into[2] = (self.0 >> 0) as u8;
        into[3] = 0; // "dummy byte", per the datasheet
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

/// `EVECommand` represents a command for an EVE controller chip.
///
/// An `EVECommand` is guaranteed to always be within the valid range of
/// values for commands, although it may not necessarily match a particular
/// valid command for the target chip.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct EVECommand(u8);

impl EVECommand {
    /// Check whether the given raw address is within the expected
    /// range for a command address, returning `true` only if so.
    pub fn is_valid(raw: u8) -> bool {
        // The two high-order bits must always be 0b01 in a command. Otherwise
        // it would be understood as either a write or read memory address.
        (raw & 0b11000000) == 0b01000000
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
        Write(EVEAddress, Vec<u8>),
        Read(EVEAddress, usize),
        Cmd(EVECommand, u8, u8),
    }

    impl MockInterface {
        const DEFAULT_MEM_VALUE: u8 = 0xff;

        pub fn new() -> Self {
            Self {
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
    }

    impl EVEInterface for MockInterface {
        type Error = ();
        fn write(&mut self, addr: EVEAddress, buf: &[u8]) -> core::result::Result<(), ()> {
            let log_buf = buf.to_vec();
            let call = MockInterfaceCall::Write(addr, log_buf);
            if let Some(fail) = self._fail {
                if fail(&call) {
                    self._calls.push(call);
                    return Err(());
                }
            }
            self._calls.push(call);

            self.setup_mem(addr, buf);
            Ok(())
        }
        fn read(&mut self, addr: EVEAddress, into: &mut [u8]) -> core::result::Result<(), ()> {
            let call = MockInterfaceCall::Read(addr, into.len());
            if let Some(fail) = self._fail {
                if fail(&call) {
                    self._calls.push(call);
                    return Err(());
                }
            }
            self._calls.push(call);

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
                MockInterfaceCall::Write(self_addr, self_data) => {
                    if let MockInterfaceCall::Write(other_addr, other_data) = other {
                        *self_addr == *other_addr && self_data.eq(other_data)
                    } else {
                        false
                    }
                }
                MockInterfaceCall::Read(self_addr, self_len) => {
                    if let MockInterfaceCall::Read(other_addr, other_len) = other {
                        *self_addr == *other_addr && *self_len == *other_len
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
