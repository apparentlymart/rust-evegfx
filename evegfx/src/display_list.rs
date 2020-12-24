use core::convert::TryFrom;
use core::fmt::Debug;
use num_enum::{IntoPrimitive, TryFromPrimitive};

/// Represents an EVE display list command.
#[derive(Copy, Clone, PartialEq)]
pub struct DLCmd(u32);

impl DLCmd {
    // The length of a display list command as stored in the EVE device's
    // display list RAM.
    pub const LENGTH: u32 = 4;

    pub const DISPLAY: Self = OpCode::DISPLAY.build(0);
    pub const CLEAR_ALL: Self = Self::clear(true, true, true);

    /// Creates a command from the raw command word given as a `u32`. It's
    /// the caller's responsibility to ensure that it's a valid encoding of
    /// a real display list command.
    pub const fn raw(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn alpha_func(func: AlphaTestFunc, ref_val: u8) -> Self {
        OpCode::ALPHA_FUNC.build((func as u32) << 8 | (ref_val as u32))
    }

    pub const fn begin(prim: GraphicsPrimitive) -> Self {
        OpCode::BEGIN.build(prim as u32)
    }

    pub const fn bitmap_ext_format(format: BitmapExtFormat) -> Self {
        OpCode::BITMAP_EXT_FORMAT.build(format as u32)
    }

    pub const fn bitmap_handle(bmp: BitmapHandle) -> Self {
        OpCode::BITMAP_HANDLE.build(bmp.0 as u32)
    }

    pub const fn bitmap_layout(format: BitmapFormat, line_stride: u16, height: u16) -> Self {
        OpCode::BITMAP_LAYOUT.build(
            (format as u32) << 19
                | (line_stride as u32 & 0b1111111111) << 9
                | (height as u32 & 0b111111111),
        )
    }

    pub const fn bitmap_layout_h(line_stride: u16, height: u16) -> Self {
        OpCode::BITMAP_LAYOUT_H.build((line_stride as u32 >> 10) << 2 | (height as u32 >> 10))
    }

    /// `bitmap_layout_pair` is a helper for calling both `bitmap_layout` and
    /// `bitmap_layout_h` with the same values, in order to set all 12 of
    /// the bits in the `line_stride` and `height` fields. Write the two
    /// commands to consecutive positions in the display list.
    pub const fn bitmap_layout_pair(
        format: BitmapFormat,
        line_stride: u16,
        height: u16,
    ) -> (Self, Self) {
        (
            Self::bitmap_layout(format, line_stride, height),
            Self::bitmap_layout_h(line_stride, height),
        )
    }

    const fn physical_bitmap_size(width: u16, height: u16) -> (u16, u16) {
        (
            if width < 2048 { width } else { 0 },
            if height < 2048 { height } else { 0 },
        )
    }

    pub const fn bitmap_size(
        width: u16,
        height: u16,
        filter: BitmapSizeFilter,
        wrap_x: BitmapWrapMode,
        wrap_y: BitmapWrapMode,
    ) -> Self {
        let (p_width, p_height) = Self::physical_bitmap_size(width, height);
        OpCode::BITMAP_SIZE.build(
            (filter as u32) << 20
                | (wrap_x as u32) << 19
                | (wrap_y as u32) << 18
                | (p_width as u32 & 0b111111111) << 9
                | (p_height as u32 & 0b111111111),
        )
    }

    pub const fn bitmap_size_h(width: u16, height: u16) -> Self {
        let (p_width, p_height) = Self::physical_bitmap_size(width, height);
        let p_width = ((p_width as u32) >> 9) & 0b11;
        let p_height = ((p_height as u32) >> 9) & 0b11;
        OpCode::BITMAP_SIZE_H.build(p_width << 9 | p_height)
    }

    /// `bitmap_size_pair` is a helper for calling both `bitmap_size` and
    /// `bitmap_size_h` with the same values, in order to set all 13 of
    /// the bits in the `width` and `height` fields. Write the two
    /// commands to consecutive positions in the display list.
    pub const fn bitmap_size_pair(
        width: u16,
        height: u16,
        filter: BitmapSizeFilter,
        wrap_x: BitmapWrapMode,
        wrap_y: BitmapWrapMode,
    ) -> (Self, Self) {
        (
            Self::bitmap_size(width, height, filter, wrap_x, wrap_y),
            Self::bitmap_size_h(width, height),
        )
    }

    pub const fn clear(color: bool, stencil: bool, tag: bool) -> Self {
        OpCode::CLEAR.build(
            if color { 0b100 } else { 0b000 }
                | if stencil { 0b010 } else { 0b000 }
                | if tag { 0b001 } else { 0b000 },
        )
    }

    pub const fn clear_color_rgb(color: crate::color::EVEColorRGB) -> Self {
        OpCode::CLEAR_COLOR_RGB
            .build((color.r as u32) << 16 | (color.b as u32) << 8 | (color.g as u32) << 0)
    }

    pub const fn clear_color_alpha(alpha: u8) -> Self {
        OpCode::CLEAR_COLOR_A.build(alpha as u32)
    }

    pub const fn clear_color_rgba_pair(color: crate::color::EVEColorRGBA) -> (Self, Self) {
        (
            Self::clear_color_rgb(color.as_rgb()),
            Self::clear_color_alpha(color.a),
        )
    }

    pub const fn display() -> Self {
        Self::DISPLAY
    }
}

/// DLBuilder is a helper for concisely building display lists. It's used only
/// in conjunction with closure-based display-list-construction functions.
pub struct DLBuilder<'a, W: DLWrite> {
    w: &'a mut W,
}

impl<'a, W: DLWrite> DLBuilder<'a, W> {
    pub(crate) fn new(writer: &'a mut W) -> Self {
        Self { w: writer }
    }

    pub fn append(&mut self, cmd: DLCmd) -> Result<(), W::Error> {
        self.w.write_dl_cmd(cmd)
    }

    pub fn raw(&mut self, raw: u32) -> Result<(), W::Error> {
        self.append(DLCmd::raw(raw))
    }

    pub fn begin(&mut self, prim: GraphicsPrimitive) -> Result<(), W::Error> {
        self.append(DLCmd::begin(prim))
    }

    pub fn clear(&mut self, color: bool, stencil: bool, tag: bool) -> Result<(), W::Error> {
        self.append(DLCmd::clear(color, stencil, tag))
    }

    pub fn clear_all(&mut self) -> Result<(), W::Error> {
        self.append(DLCmd::CLEAR_ALL)
    }

    pub fn clear_color_rgb(&mut self, color: crate::color::EVEColorRGB) -> Result<(), W::Error> {
        self.append(DLCmd::clear_color_rgb(color))
    }

    pub fn clear_color_alpha(&mut self, alpha: u8) -> Result<(), W::Error> {
        self.append(DLCmd::clear_color_alpha(alpha))
    }

    pub fn clear_color_rgba(&mut self, color: crate::color::EVEColorRGBA) -> Result<(), W::Error> {
        let cmds = DLCmd::clear_color_rgba_pair(color);
        self.append(cmds.0)?;
        self.append(cmds.1)
    }

    pub fn display(&mut self) -> Result<(), W::Error> {
        self.append(DLCmd::DISPLAY)
    }
}

pub trait DLWrite {
    type Error;

    fn write_dl_cmd(&mut self, cmd: DLCmd) -> Result<(), Self::Error>;
}

/// Each command is encoded as a four-byte value. Converting to `u32` returns
/// the raw encoding of the command, as it would be written into display
/// list memory (endianness notwithstanding).
impl Into<u32> for DLCmd {
    fn into(self) -> u32 {
        self.0
    }
}

impl Debug for DLCmd {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "DLCmd({:#010x})", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(non_camel_case_types)]
enum OpCode {
    ALPHA_FUNC = 0x09,
    BEGIN = 0x1F,
    BITMAP_EXT_FORMAT = 0x2e,
    BITMAP_HANDLE = 0x05,
    BITMAP_LAYOUT = 0x07,
    BITMAP_LAYOUT_H = 0x28,
    BITMAP_SIZE = 0x08,
    BITMAP_SIZE_H = 0x29,
    CLEAR = 0x26,
    CLEAR_COLOR_RGB = 0x02,
    CLEAR_COLOR_A = 0x0F,
    DISPLAY = 0x00,
}

impl OpCode {
    const fn shift(self) -> u32 {
        (self as u32) << 24
    }

    const fn build(self, v: u32) -> DLCmd {
        DLCmd::raw(self.shift() | v)
    }
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
#[repr(u8)]
pub enum AlphaTestFunc {
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
pub struct BitmapHandle(u8);

impl BitmapHandle {
    //// Mask representing the bits of a u8 that contribute to an EVEAddress.
    pub const MASK: u8 = 0x1f;

    /// `SCRATCH` is the bitmap handle reserved for use by some coprocessor
    /// behaviors. If you aren't using the coprocessor then there's nothing
    /// special about this handle.
    pub const SCRATCH: Self = Self::force_raw(15);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dlcmd() {
        assert_eq!(
            DLCmd::alpha_func(AlphaTestFunc::Greater, 254),
            DLCmd::raw(0x090003fe),
        );
        assert_eq!(
            DLCmd::alpha_func(AlphaTestFunc::Never, 0),
            DLCmd::raw(0x09000000),
        );
        assert_eq!(
            DLCmd::begin(GraphicsPrimitive::Bitmaps),
            DLCmd::raw(0x1f000001),
        );
        assert_eq!(
            DLCmd::begin(GraphicsPrimitive::Rects),
            DLCmd::raw(0x1f000009),
        );
        assert_eq!(
            DLCmd::bitmap_ext_format(BitmapExtFormat::ARGB1555),
            DLCmd::raw(0x2e000000),
        );
        assert_eq!(
            DLCmd::bitmap_ext_format(BitmapExtFormat::ARGB4),
            DLCmd::raw(0x2e000006),
        );
        assert_eq!(
            DLCmd::bitmap_ext_format(BitmapExtFormat::TextVGA),
            DLCmd::raw(0x2e00000a),
        );
        assert_eq!(
            DLCmd::bitmap_handle(BitmapHandle::force_raw(0)),
            DLCmd::raw(0x05000000),
        );
        assert_eq!(
            DLCmd::bitmap_handle(BitmapHandle::force_raw(15)),
            DLCmd::raw(0x0500000f),
        );
        assert_eq!(
            DLCmd::bitmap_handle(BitmapHandle::force_raw(31)),
            DLCmd::raw(0x0500001f),
        );
        assert_eq!(
            DLCmd::bitmap_layout(BitmapFormat::ARGB4, 255, 255),
            DLCmd::raw(0x0731feff),
        );
        assert_eq!(
            DLCmd::bitmap_layout(BitmapFormat::ARGB4, 1024, 768),
            DLCmd::raw(0x07300100),
        );
        assert_eq!(DLCmd::bitmap_layout_h(255, 255), DLCmd::raw(0x28000000));
        assert_eq!(DLCmd::bitmap_layout_h(1024, 768), DLCmd::raw(0x28000004));
        assert_eq!(
            DLCmd::bitmap_layout_pair(BitmapFormat::ARGB4, 255, 255),
            (DLCmd::raw(0x0731feff), DLCmd::raw(0x28000000)),
        );
        assert_eq!(
            DLCmd::bitmap_layout_pair(BitmapFormat::ARGB4, 1024, 768),
            (DLCmd::raw(0x07300100), DLCmd::raw(0x28000004)),
        );
        assert_eq!(
            DLCmd::bitmap_size(
                255,
                255,
                BitmapSizeFilter::Nearest,
                BitmapWrapMode::Border,
                BitmapWrapMode::Border
            ),
            DLCmd::raw(0x0801feff),
        );
        assert_eq!(
            DLCmd::bitmap_size(
                2048,
                2048,
                BitmapSizeFilter::Nearest,
                BitmapWrapMode::Border,
                BitmapWrapMode::Border
            ),
            DLCmd::raw(0x08000000),
        );
        assert_eq!(
            DLCmd::bitmap_size(
                1024,
                768,
                BitmapSizeFilter::Nearest,
                BitmapWrapMode::Border,
                BitmapWrapMode::Border
            ),
            DLCmd::raw(0x08000100),
        );
        assert_eq!(
            DLCmd::bitmap_size(
                1,
                1,
                BitmapSizeFilter::Bilinear,
                BitmapWrapMode::Border,
                BitmapWrapMode::Border
            ),
            DLCmd::raw(0x08100201),
        );
        assert_eq!(
            DLCmd::bitmap_size(
                1,
                1,
                BitmapSizeFilter::Nearest,
                BitmapWrapMode::Repeat,
                BitmapWrapMode::Border
            ),
            DLCmd::raw(0x08080201),
        );
        assert_eq!(
            DLCmd::bitmap_size(
                1,
                1,
                BitmapSizeFilter::Nearest,
                BitmapWrapMode::Border,
                BitmapWrapMode::Repeat
            ),
            DLCmd::raw(0x08040201),
        );
        assert_eq!(
            DLCmd::bitmap_size_pair(
                255,
                255,
                BitmapSizeFilter::Nearest,
                BitmapWrapMode::Border,
                BitmapWrapMode::Border
            ),
            (DLCmd::raw(0x0801feff), DLCmd::raw(0x29000000))
        );
        assert_eq!(
            DLCmd::bitmap_size_pair(
                2048,
                2048,
                BitmapSizeFilter::Nearest,
                BitmapWrapMode::Border,
                BitmapWrapMode::Border
            ),
            (DLCmd::raw(0x08000000), DLCmd::raw(0x29000000)),
        );
        assert_eq!(
            DLCmd::bitmap_size_pair(
                1024,
                768,
                BitmapSizeFilter::Nearest,
                BitmapWrapMode::Border,
                BitmapWrapMode::Border
            ),
            (DLCmd::raw(0x08000100), DLCmd::raw(0x29000401)),
        );
    }
}
