//! Data types for various arguments to display list commands.

use core::convert::{From, TryFrom};
use num_enum::{IntoPrimitive, TryFromPrimitive};

/// Test function options for both alpha test and stencil test during drawing
/// operations. This is used by both the `alpha_test` and `stencil_test`
/// methods.
#[derive(TryFromPrimitive, IntoPrimitive, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum TestFunc {
    Never = 0,
    Less = 1,
    LEqual = 2,
    Greater = 3,
    GEqual = 4,
    Equal = 5,
    NotEqual = 6,
    Always = 7,
}

#[derive(TryFromPrimitive, IntoPrimitive, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum GraphicsPrimitive {
    Bitmaps = 1,
    Points = 2,
    Lines = 3,
    LineStrip = 4,
    EdgeStripR = 5,
    EdgeStripL = 6,
    EdgeStripA = 7,
    EdgeStripB = 8,
    Rects = 9,
}

#[derive(TryFromPrimitive, IntoPrimitive, Clone, Copy, PartialEq)]
#[repr(u16)]
pub enum BitmapExtFormat {
    ARGB1555 = 0,
    L1 = 1,
    L4 = 2,
    L8 = 3,
    RGB332 = 4,
    ARGB2 = 5,
    ARGB4 = 6,
    RGB565 = 7,
    Text8x8 = 9,
    TextVGA = 10,
    Bargraph = 11,
    Paletted565 = 14,
    Paletted4444 = 15,
    Paletted8 = 16,
    L2 = 17,
    CompressedRGBAASTC4x4KHR = 37808,
    CompressedRGBAASTC5x4KHR = 37809,
    CompressedRGBAASTC5x5KHR = 37810,
    CompressedRGBAASTC6x5KHR = 37811,
    CompressedRGBAASTC6x6KHR = 37812,
    CompressedRGBAASTC8x5KHR = 37813,
    CompressedRGBAASTC8x6KHR = 37814,
    CompressedRGBAASTC8x8KHR = 37815,
    CompressedRGBAASTC10x5KHR = 37816,
    CompressedRGBAASTC10x6KHR = 37817,
    CompressedRGBAASTC10x8KHR = 37818,
    CompressedRGBAASTC10x10KHR = 37819,
    CompressedRGBAASTC12x10KHR = 37820,
    CompressedRGBAASTC12x12KHR = 37821,
}

#[derive(TryFromPrimitive, IntoPrimitive, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum BitmapFormat {
    ARGB1555 = 0,
    L1 = 1,
    L4 = 2,
    L8 = 3,
    RGB332 = 4,
    ARGB2 = 5,
    ARGB4 = 6,
    RGB565 = 7,
    Text8x8 = 9,
    TextVGA = 10,
    Bargraph = 11,
    Paletted565 = 14,
    Paletted4444 = 15,
    Paletted8 = 16,
    L2 = 17,
    GLFormat = 31,
}

impl TryFrom<BitmapExtFormat> for BitmapFormat {
    type Error = ();
    fn try_from(ext: BitmapExtFormat) -> core::result::Result<Self, ()> {
        let raw = ext as u16;
        if raw > 17 {
            return Err(());
        }
        match BitmapFormat::try_from(raw as u8) {
            Ok(v) => Ok(v),
            Err(_) => Err(()),
        }
    }
}

impl TryFrom<BitmapFormat> for BitmapExtFormat {
    type Error = ();
    fn try_from(fmt: BitmapFormat) -> core::result::Result<Self, ()> {
        let raw = fmt as u8;
        if raw > 7 {
            // The first 17 formats are common, but the others are not
            return Err(());
        }
        match BitmapExtFormat::try_from(raw as u16) {
            Ok(v) => Ok(v),
            Err(_) => Err(()),
        }
    }
}

/// `BitmapHandle` is a display list bitmap handle, numbered between zero and
/// 31.
#[derive(Copy, Clone, PartialEq)]
pub struct BitmapHandle(pub(crate) u8);

impl BitmapHandle {
    //// Mask representing the bits of a u8 that contribute to an EVEAddress.
    pub const MASK: u8 = 0x1f;

    /// `DEFAULT_SCRATCH` is the bitmap handle assigned by default for use by
    /// some coprocessor behaviors. If you aren't using the coprocessor, or
    /// if you've configured the coprocessor to use a different handle for
    /// its internal work, then there's nothing special about this handle.
    pub const DEFAULT_SCRATCH: Self = Self::force_raw(15);

    /// Test whether the given raw value is within the expected
    /// range for a bitmap handle, returning `true` only if so.
    pub const fn is_valid(raw: u8) -> bool {
        // Only the lowest 22 bits may be nonzero.
        (raw & Self::MASK) == 0
    }

    /// Turns the given raw value into a valid BitmapHandle by masking
    /// out the bits that must always be zero for a valid handle.
    ///
    /// This is intended primarily for initializing global constants
    /// representing well-known bitmap handles in your program. If you're
    /// working with a dynamically-derived address value then better to use the
    /// `TryFrom<u8>` implementation to get an error if the value is out of
    /// range.
    pub const fn force_raw(raw: u8) -> Self {
        Self(raw & Self::MASK)
    }

    /// Returns `true` if the handle is one of the ones that has a preassigned
    /// special purpose. These special purposes are optional but you may wish
    /// to prefer using non-special handles if any are available.
    pub const fn is_special(self) -> bool {
        self.0 >= 15
    }
}

impl TryFrom<u8> for BitmapHandle {
    type Error = ();

    fn try_from(raw: u8) -> Result<Self, Self::Error> {
        if Self::is_valid(raw) {
            Ok(Self(raw))
        } else {
            Err(())
        }
    }
}

impl From<BitmapHandle> for u8 {
    fn from(bmp: BitmapHandle) -> u8 {
        bmp.0
    }
}

impl From<BitmapHandle> for u32 {
    fn from(bmp: BitmapHandle) -> u32 {
        bmp.0 as u32
    }
}

#[derive(TryFromPrimitive, IntoPrimitive, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum BitmapSizeFilter {
    Nearest = 0,
    Bilinear = 1,
}

#[derive(TryFromPrimitive, IntoPrimitive, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum BitmapWrapMode {
    Border = 0,
    Repeat = 1,
}
