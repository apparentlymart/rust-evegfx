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
#[derive(Clone, Copy, PartialEq, PartialOrd)]
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
#[derive(Clone, Copy, PartialEq, PartialOrd)]
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
