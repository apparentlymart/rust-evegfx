use core::marker::PhantomData;

/// A pointer to a memory address within a particular memory region identified
/// by type parameter `R`.
///
/// Pointers are parameterized by memory region so that other parts of this
/// library which consume pointers can statically constrain what memory regions
/// they are able to refer to.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Ptr<R: MemoryRegion> {
    addr: u32,
    _region: PhantomData<R>,
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
    pub fn new(self, offset: u32) -> Self {
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

impl<R: MemoryRegion> core::ops::Add<i32> for Ptr<R> {
    type Output = Self;

    fn add(self, offset: i32) -> Self {
        R::ptr(self.to_raw() + (offset as u32))
    }
}

impl<R: MemoryRegion> core::ops::Sub<i32> for Ptr<R> {
    type Output = Self;

    fn sub(self, offset: i32) -> Self {
        R::ptr(self.to_raw() - (offset as u32))
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

/// A trait implemented by all memory regions that [`Ptr`](Ptr) instances can
/// refer to.
///
/// It doesn't make sense to implement this trait outside of the `evegfx`
/// crate. It is implemented by EVE-model-specific APIs elsewhere in this
/// crate. Within the context of a particular model none of the available
/// memory regions may overlap.
///
/// Memory regions exist only at compile time, as a facility to have the
/// Rust type system help ensure valid use of pointers. At runtime we
/// deal only in absolute addresses represented as u32.
pub trait MemoryRegion: core::marker::Sized {
    const BASE_ADDR: u32;
    const LENGTH: u32;
    const DEBUG_NAME: &'static str;

    /// Creates a pointer in the selected memory region.
    ///
    /// The given value is interpreted as an offset into the memory region,
    /// modulo the size of the region.
    #[inline]
    fn ptr(raw: u32) -> Ptr<Self> {
        Ptr {
            addr: Self::BASE_ADDR + (raw % Self::LENGTH),
            _region: PhantomData,
        }
    }
}

pub trait MainMem: MemoryRegion + HostAccessible {}

pub trait FontMem: MemoryRegion + HostAccessible {}

pub trait DisplayListMem: MemoryRegion + HostAccessible {}

pub trait RegisterMem: MemoryRegion + HostAccessible {}

pub trait CommandMem: MemoryRegion + HostAccessible {}

pub trait CommandErrMem: MemoryRegion + HostAccessible {}

/// Implemented by memory regions that can be accessed indirectly via the
/// `CMD_FLASH...` family of coprocessor commands.
pub trait ExtFlashMem: MemoryRegion {}

/// Implemented by memory regions that can be directly read or written by
/// the host controller. Memory regions implementing this trait may only
/// use the lower 22 bits of the address space, with the topmost 10 bits
/// always set to zero.
pub trait HostAccessible: MemoryRegion {}
