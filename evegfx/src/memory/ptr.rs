use super::region::*;
use core::marker::PhantomData;

/// A pointer to a memory address within a particular memory region identified
/// by type parameter `R`.
///
/// Pointers are parameterized by memory region so that other parts of this
/// library which consume pointers can statically constrain what memory regions
/// they are able to refer to.
#[derive(Copy, Clone)]
pub struct Ptr<R: MemoryRegion> {
    pub(crate) addr: u32,
    pub(crate) _region: PhantomData<R>,
}

/// General API for pointers across all memory regions.
impl<R: MemoryRegion> Ptr<R> {
    /// Constructs a new pointer from the given raw address.
    ///
    /// Note that a pointer always belongs to a memory region, but there's
    /// no argument here to select one. Instead, we typically rely on
    /// type inference to select one, by using the result in a context which
    /// implies a particular memory region.
    ///
    /// The given offset is interpreted as an offset into the selected
    /// memory region, modulo the region length. See
    /// [`MemoryRegion::ptr`](MemoryRegion::ptr) for more information.
    #[inline]
    pub fn new(offset: u32) -> Self {
        R::ptr(offset)
    }

    /// Returns the absolute address of the pointer.
    #[inline]
    pub fn to_raw(self) -> u32 {
        self.addr
    }

    /// Returns the offset of the pointer relative to its containing memory
    /// region.
    #[inline]
    pub fn to_raw_offset(self) -> u32 {
        self.addr - R::BASE_ADDR
    }
}

impl<R: MemoryRegion + HostAccessible> Ptr<R> {
    /// Write the three bytes needed to form a "write memory" header
    /// for the address into the given bytes. This is a helper for
    /// physical implementations that need to construct a message
    /// buffer to transmit to the real chip, e.g. via SPI.
    pub fn build_spi_write_header(self, into: &mut [u8; 3]) {
        into[0] = (((self.addr >> 16) & 0b00111111) | 0b10000000) as u8;
        into[1] = (self.addr >> 8) as u8;
        into[2] = (self.addr >> 0) as u8;
    }

    /// Write the four bytes needed to form a "read memory" header
    /// for the address into the given bytes. This is a helper for
    /// physical implementations that need to construct a message
    /// buffer to transmit to the real chip, e.g. via SPI.
    pub fn build_spi_read_header(self, into: &mut [u8; 4]) {
        into[0] = ((self.addr >> 16) & 0b00111111) as u8;
        into[1] = (self.addr >> 8) as u8;
        into[2] = (self.addr >> 0) as u8;
        into[3] = 0; // "dummy byte", per the datasheet
    }
}

impl<R1: MemoryRegion, R2: MemoryRegion<Model = R1::Model>> core::cmp::PartialEq<Ptr<R2>>
    for Ptr<R1>
{
    fn eq(&self, other: &Ptr<R2>) -> bool {
        self.addr == other.addr
    }
}

impl<R: MemoryRegion> core::cmp::Eq for Ptr<R> {}

impl<R1: MemoryRegion, R2: MemoryRegion<Model = R1::Model>> core::cmp::PartialOrd<Ptr<R2>>
    for Ptr<R1>
{
    fn partial_cmp(&self, other: &Ptr<R2>) -> core::option::Option<core::cmp::Ordering> {
        if self.addr == other.addr {
            Some(core::cmp::Ordering::Equal)
        } else if self.addr < other.addr {
            Some(core::cmp::Ordering::Less)
        } else {
            Some(core::cmp::Ordering::Greater)
        }
    }
}

impl<R: MemoryRegion> core::cmp::Ord for Ptr<R> {
    fn cmp(&self, other: &Ptr<R>) -> core::cmp::Ordering {
        if self.addr == other.addr {
            core::cmp::Ordering::Equal
        } else if self.addr < other.addr {
            core::cmp::Ordering::Less
        } else {
            core::cmp::Ordering::Greater
        }
    }
}

impl<R: MemoryRegion> core::ops::Add<i32> for Ptr<R> {
    type Output = Self;

    fn add(self, offset: i32) -> Self {
        R::ptr(self.to_raw() + (offset as u32))
    }
}

impl<R: MemoryRegion> core::ops::Add<u32> for Ptr<R> {
    type Output = Self;

    fn add(self, offset: u32) -> Self {
        R::ptr(self.to_raw() + offset)
    }
}

impl<R: MemoryRegion> core::ops::AddAssign<i32> for Ptr<R> {
    fn add_assign(&mut self, offset: i32) {
        *self = R::ptr(self.to_raw() + (offset as u32))
    }
}

impl<R: MemoryRegion> core::ops::AddAssign<u32> for Ptr<R> {
    fn add_assign(&mut self, offset: u32) {
        *self = R::ptr(self.to_raw() + offset)
    }
}

impl<R: MemoryRegion> core::ops::Sub<i32> for Ptr<R> {
    type Output = Self;

    fn sub(self, offset: i32) -> Self {
        R::ptr(self.to_raw() - (offset as u32))
    }
}

impl<R: MemoryRegion> core::ops::Sub<u32> for Ptr<R> {
    type Output = Self;

    fn sub(self, offset: u32) -> Self {
        R::ptr(self.to_raw() - offset)
    }
}

impl<R: MemoryRegion> core::ops::SubAssign<i32> for Ptr<R> {
    fn sub_assign(&mut self, offset: i32) {
        *self = R::ptr(self.to_raw() - (offset as u32))
    }
}

impl<R: MemoryRegion> core::ops::SubAssign<u32> for Ptr<R> {
    fn sub_assign(&mut self, offset: u32) {
        *self = R::ptr(self.to_raw() - offset)
    }
}

impl<R: MemoryRegion> core::fmt::Debug for Ptr<R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Ptr {{ addr: {:#010x?} /*{}*/ }}",
            self.addr,
            R::DEBUG_NAME
        )
    }
}

impl<R: MemoryRegion> core::fmt::Display for Ptr<R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#010x?}", self.addr)
    }
}
