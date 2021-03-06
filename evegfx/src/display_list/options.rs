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

impl BitmapFormat {
    /// Computes the most compact possible stride for an image of the given
    /// width (in pixels) in the associated bitmap format.
    ///
    /// A bitmap might have a larger stride than returned by this function,
    /// whether due to its representation in memory having gaps or due to
    /// there being multiple "cells" associated with the bitmap, each of
    /// which is of the indicated width. In the latter case, you can multiply
    /// the `minimum_stride` result by the number of cells to find the
    /// true stride, assuming that the cells are stored compactly.
    ///
    /// For bitmap formats with fewer than eight bits per pixel, the result
    /// is automatically padded to a round number of bytes, as expected by
    /// the display engine.
    ///
    /// For the bitmap formats that are based on character cells rather than
    /// on pixels, the width should be given in character cells, and the
    /// result will be the minimum stride for that given number of character
    /// cells.
    ///
    /// `BitmapFormat::GLFormat` is not supported for this method, because
    /// it's not a real format but rather just a marker that the format is
    /// specified as a `BitmapExtFormat` instead. `BitmapFormat::GLFormat`
    /// therefore always returns a stride of zero, as an invalid placeholder.
    pub fn minimum_stride(self, width: u32) -> u32 {
        match self {
            Self::ARGB1555 => width * 2,
            Self::L8 => width,
            Self::RGB332 => width,
            Self::ARGB2 => width,
            Self::ARGB4 => width * 2,
            Self::RGB565 => width * 2,
            Self::Text8x8 => width,     // width in character cells
            Self::TextVGA => width * 2, // width in character cells
            Self::Bargraph => width,
            Self::Paletted565 => width,
            Self::Paletted4444 => width,
            Self::Paletted8 => width,
            // The remaining formats are a little more awkward because
            // they might need padding to achieve byte alignment.
            Self::L1 => Self::bytes_for_bits(width),
            Self::L4 => Self::bytes_for_bits(width * 4),
            Self::L2 => Self::bytes_for_bits(width * 2),
            Self::GLFormat => 0,
        }
    }

    fn bytes_for_bits(bits: u32) -> u32 {
        // Under integer arithmetic, this rounds up to the nearest multiple
        // of eight.
        (bits + 7) / 8
    }

    /// Returns `true` if the format requires an extended format to be
    /// specified and is therefore not self-sufficient.
    ///
    /// If you intend to represent extended formats then you should use
    /// `BitmapExtFormat` instead. Converting a `BitmapExtFormat` to
    /// `BitmapFormat` will return a format which returns `true` from this
    /// method if the extended format doesn't correspond with one of the
    /// base formats.
    pub fn needs_ext_format(self) -> bool {
        match self {
            Self::GLFormat => true,
            _ => false,
        }
    }
}

impl From<BitmapExtFormat> for BitmapFormat {
    fn from(ext: BitmapExtFormat) -> Self {
        let raw = ext as u16;
        if raw > 17 {
            return Self::GLFormat;
        }
        match BitmapFormat::try_from(raw as u8) {
            Ok(v) => v,
            Err(_) => unreachable!(),
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum BitmapSwizzleSource {
    Zero = 0,
    One = 1,
    Red = 2,
    Green = 3,
    Blue = 4,
    Alpha = 5,
}

#[derive(Debug, Copy, Clone)]
pub struct BitmapSwizzle {
    pub r: BitmapSwizzleSource,
    pub g: BitmapSwizzleSource,
    pub b: BitmapSwizzleSource,
    pub a: BitmapSwizzleSource,
}

impl BitmapSwizzle {
    pub fn as_raw(&self) -> u32 {
        (self.r as u32) << 9 | (self.g as u32) << 6 | (self.b as u32) << 3 | (self.a as u32)
    }
}

impl Default for BitmapSwizzle {
    fn default() -> Self {
        Self {
            r: BitmapSwizzleSource::Red,
            g: BitmapSwizzleSource::Green,
            b: BitmapSwizzleSource::Blue,
            a: BitmapSwizzleSource::Alpha,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum BlendFunc {
    Zero = 0,
    One = 1,
    SrcAlpha = 2,
    DstAlpha = 3,
    OneMinusSrcAlpha = 4,
    OneMinusDstAlpha = 5,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ColorMask(u8);

impl ColorMask {
    const RED_MASK: u8 = 0b1000;
    const GREEN_MASK: u8 = 0b0100;
    const BLUE_MASK: u8 = 0b0010;
    const ALPHA_MASK: u8 = 0b0001;

    pub const fn new(red: bool, green: bool, blue: bool, alpha: bool) -> Self {
        let mut ret = Self(0);
        if red {
            ret = ret.with_red();
        }
        if green {
            ret = ret.with_green();
        }
        if blue {
            ret = ret.with_blue();
        }
        if alpha {
            ret = ret.with_alpha();
        }
        ret
    }

    pub const fn with_red(self) -> Self {
        Self(self.0 | Self::RED_MASK)
    }

    pub const fn without_red(self) -> Self {
        Self(self.0 & !Self::RED_MASK)
    }

    pub const fn with_green(self) -> Self {
        Self(self.0 | Self::GREEN_MASK)
    }

    pub const fn without_green(self) -> Self {
        Self(self.0 & !Self::GREEN_MASK)
    }

    pub const fn with_blue(self) -> Self {
        Self(self.0 | Self::BLUE_MASK)
    }

    pub const fn without_blue(self) -> Self {
        Self(self.0 & !Self::BLUE_MASK)
    }

    pub const fn with_alpha(self) -> Self {
        Self(self.0 | Self::ALPHA_MASK)
    }

    pub const fn without_alpha(self) -> Self {
        Self(self.0 & !Self::ALPHA_MASK)
    }

    pub const fn to_raw(self) -> u8 {
        self.0
    }
}

impl Default for ColorMask {
    fn default() -> Self {
        Self(0b1111) // All are enabled by default
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum StencilOp {
    Zero = 0,
    Keep = 1,
    Replace = 2,
    Incr = 3,
    Decr = 4,
    Invert = 5,
}

impl StencilOp {
    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum VertexFormat {
    Whole = 0,
    Half = 1,
    Quarter = 2,
    Eighth = 3,
    Sixteenth = 4,
}

impl VertexFormat {
    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

/// A matrix coefficient for use with the bitmap transform matrix.
#[derive(Copy, Clone, PartialEq)]
pub struct MatrixCoeff(pub(crate) u32);

impl MatrixCoeff {
    const P_MASK: u32 = 0x10000;
    const SCALE_8_8: f32 = 256.0;
    const SCALE_1_15: f32 = 32768.0;

    pub const ZERO: Self = Self::new_int(0);
    pub const ONE: Self = Self::new_int(1);

    /// Creates a matrix coefficient with a whole number value.
    pub const fn new_int(v: i8) -> Self {
        let enc = (((v as i16) << 8) as u16) as u32;
        MatrixCoeff(enc)
    }

    /// Creates a matrix coefficient with an approximation of the given float
    /// value, with an eight-bit whole number part and an eight-bit fractional
    /// part.
    ///
    /// This has the same range as `new_88`.
    pub fn new_f32_approx_8_8(v: f32) -> Self {
        let enc = ((v * Self::SCALE_8_8) as i16) as u16 as u32;
        MatrixCoeff(enc)
    }

    /// Creates a matrix coefficient with an approximation of the given float
    /// value, with a one-bit whole number part and a fifteen-bit fractional
    /// part.
    ///
    /// This form gives better precision than `new_f32_approx_8_8` for values
    /// between -1 and 1 exclusive.
    ///
    /// This has the same range as `new_8_8`.
    pub fn new_f32_approx_1_15(v: f32) -> Self {
        let enc = ((v * Self::SCALE_1_15) as i16) as u16 as u32;
        MatrixCoeff(enc | Self::P_MASK)
    }

    /// Creates a matrix coefficient with an eight-bit whole number part and
    /// an eight-bit fractional part.
    ///
    /// For example, use `MatrixCoeff::new_8_8(0, 5)` to represent the number
    /// 0.5, or `MatrixCoeff::new_8_8(2, 0)` to represent the number 2.
    pub const fn new_8_8(whole: i8, frac: u8) -> Self {
        let whole_part = Self::new_int(whole);
        // The fractional part is now XOR into the lower eight bits, which
        // means it will be inverted if whole_part is already a negative
        // number.
        MatrixCoeff(whole_part.0 ^ (frac as u32))
    }

    /// Creates a matrix coefficient with a sign and a 15-bit fractional part.
    ///
    /// For example, use `MatrixCoeff::new_1_15(5)` to represent the number
    /// 0.5, or `MatrixCoeff::new_1_15(-25)` to represent the number -0.25.
    pub const fn new_1_15(frac: i16) -> Self {
        // We shift up once and down once to discard the top-most bit, but
        // we're doing both of these with signed values so it will still
        // preserve the sign.
        MatrixCoeff(((frac << 1) >> 1) as u32 | Self::P_MASK)
    }

    /// Returns true if the value is encoded in the 8.8 format, where both
    /// the whole number and fractional parts are eight bits in length.
    pub const fn is_8_8(self) -> bool {
        (self.0 & Self::P_MASK) == 0
    }

    /// Returns true if the value is encoded in the 1.15 format, where there
    /// are 15 bits representing the fractional part and only one bit
    /// representing the whole number part.
    pub const fn is_1_15(self) -> bool {
        (self.0 & Self::P_MASK) != 0
    }

    /// Returns the value that the result of `to_raw_value` should be divided
    /// by in order to recover the intended value.
    pub const fn scale(self) -> f32 {
        if self.is_8_8() {
            Self::SCALE_8_8
        } else {
            Self::SCALE_1_15
        }
    }

    /// Returns the number of bits in the result of `to_raw_value` that
    /// represent the fractional part.
    ///
    /// Shifting right by this amount will recover the integer part of the
    /// value.
    pub const fn shift(self) -> usize {
        if self.is_8_8() {
            8
        } else {
            15
        }
    }

    pub(crate) const fn to_raw(self) -> u32 {
        self.0
    }

    /// Returns the raw 16-bit encoding of the value.
    ///
    /// The structure of this value depends on the encoding format; use
    /// `is_1_15` and/or `is_8_8` to find the encoding format. Both formats
    /// cause the result to be scaled by a power of two, so you can reverse
    /// the encoding by dividing by that value.
    pub const fn to_raw_value(self) -> i16 {
        self.0 as i16 // discard the P flag in bit 17
    }

    /// Returns the integer part of the value, discarding the fractional part.
    ///
    /// This is the `floor` operation, rounding down towards zero.
    pub const fn to_i8(self) -> i8 {
        (self.to_raw_value() >> self.shift()) as i8
    }

    /// Returns a floating-point interpretation of the value.
    pub fn to_f32(self) -> f32 {
        let raw = self.to_raw_value() as f32;
        raw / self.scale()
    }
}
impl From<f32> for MatrixCoeff {
    fn from(v: f32) -> Self {
        // We'll select the 1.15 encoding if the given number is within
        // a range that seems like it would benefit.
        if v < 1.0 && v >= -1.0 {
            Self::new_f32_approx_1_15(v)
        } else {
            Self::new_f32_approx_8_8(v)
        }
    }
}
impl From<MatrixCoeff> for f32 {
    fn from(v: MatrixCoeff) -> f32 {
        v.to_f32()
    }
}
impl From<i8> for MatrixCoeff {
    fn from(v: i8) -> Self {
        Self::new_int(v)
    }
}
impl From<MatrixCoeff> for i8 {
    fn from(v: MatrixCoeff) -> i8 {
        v.to_i8()
    }
}

/// A 3 by 2 bitmap transformation matrix.
///
/// Methods that expect matrices usually accept any type that can convert to
/// a matrix, so if it's clear from context that the value is a matrix then
/// you can just pass a representation based on a tuple of two tuples with
/// three coefficients each, representing the rows and columns of the matrix.
pub struct Matrix3x2(
    pub(crate) (MatrixCoeff, MatrixCoeff, MatrixCoeff),
    pub(crate) (MatrixCoeff, MatrixCoeff, MatrixCoeff),
);

impl Matrix3x2 {
    pub const IDENTITY: Self = Self(
        (MatrixCoeff::ONE, MatrixCoeff::ZERO, MatrixCoeff::ZERO),
        (MatrixCoeff::ZERO, MatrixCoeff::ONE, MatrixCoeff::ZERO),
    );
}

impl<A, B, C, D, E, F> From<((A, B, C), (D, E, F))> for Matrix3x2
where
    A: Into<MatrixCoeff>,
    B: Into<MatrixCoeff>,
    C: Into<MatrixCoeff>,
    D: Into<MatrixCoeff>,
    E: Into<MatrixCoeff>,
    F: Into<MatrixCoeff>,
{
    fn from(coeffs: ((A, B, C), (D, E, F))) -> Self {
        Self(
            (coeffs.0 .0.into(), coeffs.0 .1.into(), coeffs.0 .2.into()),
            (coeffs.1 .0.into(), coeffs.1 .1.into(), coeffs.1 .2.into()),
        )
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
