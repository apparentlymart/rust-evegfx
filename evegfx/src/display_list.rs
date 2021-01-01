//! Representations of display list commands.

pub mod options;

use crate::graphics::{Vertex2F, Vertex2II, RGB, RGBA};
use core::fmt::Debug;

/// Represents an EVE display list command.
#[derive(Copy, Clone, PartialEq)]
pub struct DLCmd(u32);

impl DLCmd {
    // The length of a display list command as stored in the EVE device's
    // display list RAM.
    pub const LENGTH: u32 = 4;

    pub const DISPLAY: Self = OpCode::DISPLAY.build(0);
    pub const END: Self = OpCode::END.build(0);
    pub const CLEAR_ALL: Self = Self::clear(true, true, true);

    /// Creates a command from the raw command word given as a `u32`. It's
    /// the caller's responsibility to ensure that it's a valid encoding of
    /// a real display list command.
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn as_raw(&self) -> u32 {
        self.0
    }

    pub const fn alpha_func(func: options::TestFunc, ref_val: u8) -> Self {
        OpCode::ALPHA_FUNC.build((func as u32) << 8 | (ref_val as u32))
    }

    pub const fn begin(prim: options::GraphicsPrimitive) -> Self {
        OpCode::BEGIN.build(prim as u32)
    }

    pub const fn bitmap_ext_format(format: options::BitmapExtFormat) -> Self {
        OpCode::BITMAP_EXT_FORMAT.build(format as u32)
    }

    pub const fn bitmap_handle(bmp: options::BitmapHandle) -> Self {
        OpCode::BITMAP_HANDLE.build(bmp.0 as u32)
    }

    pub const fn bitmap_layout(
        format: options::BitmapFormat,
        line_stride: u16,
        height: u16,
    ) -> Self {
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
        format: options::BitmapFormat,
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
        filter: options::BitmapSizeFilter,
        wrap_x: options::BitmapWrapMode,
        wrap_y: options::BitmapWrapMode,
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
        filter: options::BitmapSizeFilter,
        wrap_x: options::BitmapWrapMode,
        wrap_y: options::BitmapWrapMode,
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

    pub const fn clear_color_rgb(color: RGB) -> Self {
        OpCode::CLEAR_COLOR_RGB
            .build((color.r as u32) << 16 | (color.g as u32) << 8 | (color.b as u32) << 0)
    }

    pub const fn clear_color_alpha(alpha: u8) -> Self {
        OpCode::CLEAR_COLOR_A.build(alpha as u32)
    }

    pub const fn clear_color_rgba_pair(color: RGBA) -> (Self, Self) {
        (
            Self::clear_color_rgb(color.as_rgb()),
            Self::clear_color_alpha(color.a),
        )
    }

    pub const fn display() -> Self {
        Self::DISPLAY
    }

    pub const fn end() -> Self {
        Self::END
    }

    pub const fn point_size(size: u16) -> Self {
        const MASK: u32 = 0b0000111111111111;
        OpCode::POINT_SIZE.build(size as u32 & MASK)
    }

    pub fn vertex_2f<Pos: Into<Vertex2F>>(pos: Pos) -> Self {
        let pos: Vertex2F = pos.into();
        OpCode::VERTEX2F.build((pos.x as u32) << 15 | (pos.y as u32))
    }

    pub fn vertex_2ii<Pos: Into<Vertex2II>>(pos: Pos) -> Self {
        let pos: Vertex2II = pos.into();
        OpCode::VERTEX2II.build((pos.x as u32) << 21 | (pos.y as u32) << 12)
    }
}

/// Trait implemented by objects that can append display list commands to
/// a display list.
///
/// Implementers usually implement only `append_raw_command`, and take the
/// default implementations of all of the other methods.
pub trait Builder {
    type Error;

    fn append_raw_command(&mut self, raw: u32) -> Result<(), Self::Error>;

    fn append_command(&mut self, cmd: DLCmd) -> Result<(), Self::Error> {
        self.append_raw_command(cmd.as_raw())
    }

    fn begin(&mut self, prim: options::GraphicsPrimitive) -> Result<(), Self::Error> {
        self.append_command(DLCmd::begin(prim))
    }

    fn clear(&mut self, color: bool, stencil: bool, tag: bool) -> Result<(), Self::Error> {
        self.append_command(DLCmd::clear(color, stencil, tag))
    }

    fn clear_all(&mut self) -> Result<(), Self::Error> {
        self.append_command(DLCmd::CLEAR_ALL)
    }

    fn clear_color_rgb(&mut self, color: RGB) -> Result<(), Self::Error> {
        self.append_command(DLCmd::clear_color_rgb(color))
    }

    fn clear_color_alpha(&mut self, alpha: u8) -> Result<(), Self::Error> {
        self.append_command(DLCmd::clear_color_alpha(alpha))
    }

    fn clear_color_rgba(&mut self, color: RGBA) -> Result<(), Self::Error> {
        let cmds = DLCmd::clear_color_rgba_pair(color);
        self.append_command(cmds.0)?;
        self.append_command(cmds.1)
    }

    fn display(&mut self) -> Result<(), Self::Error> {
        self.append_command(DLCmd::DISPLAY)
    }

    fn end(&mut self) -> Result<(), Self::Error> {
        self.append_command(DLCmd::END)
    }

    fn point_size(&mut self, size: u16) -> Result<(), Self::Error> {
        self.append_command(DLCmd::point_size(size))
    }

    fn vertex_2f<Pos: Into<Vertex2F>>(&mut self, pos: Pos) -> Result<(), Self::Error> {
        self.append_command(DLCmd::vertex_2f(pos))
    }

    fn vertex_2ii<Pos: Into<Vertex2II>>(&mut self, pos: Pos) -> Result<(), Self::Error> {
        self.append_command(DLCmd::vertex_2ii(pos))
    }
}

/// An implementation of `Builder` that _only_ has the display
/// list building functionality, wrapping another object that implements the
/// trait, for situations where it would be inappropriate to use other
/// functionality of the wrapped object while building a display list.
pub struct JustBuilder<'a, W: Builder> {
    w: &'a mut W,
}

impl<'a, W: Builder> JustBuilder<'a, W> {
    fn new(w: &'a mut W) -> Self {
        Self { w: w }
    }
}

impl<'a, W: Builder> Builder for JustBuilder<'a, W> {
    type Error = W::Error;

    fn append_raw_command(&mut self, raw: u32) -> core::result::Result<(), W::Error> {
        self.w.append_raw_command(raw)
    }
}

pub fn just_builder<'a, W: Builder>(wrapped: &'a mut W) -> JustBuilder<'a, W> {
    JustBuilder::new(wrapped)
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
    END = 0x21,
    POINT_SIZE = 0x0d,
    VERTEX2F = 0b01000000,  // This opcode is packed into the two MSB
    VERTEX2II = 0b10000000, // This opcode is packed into the two MSB
}

impl OpCode {
    const fn shift(self) -> u32 {
        (self as u32) << 24
    }

    const fn build(self, v: u32) -> DLCmd {
        DLCmd::from_raw(self.shift() | v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dlcmd() {
        assert_eq!(
            DLCmd::alpha_func(options::TestFunc::Greater, 254),
            DLCmd::from_raw(0x090003fe),
        );
        assert_eq!(
            DLCmd::alpha_func(options::TestFunc::Never, 0),
            DLCmd::from_raw(0x09000000),
        );
        assert_eq!(
            DLCmd::begin(options::GraphicsPrimitive::Bitmaps),
            DLCmd::from_raw(0x1f000001),
        );
        assert_eq!(
            DLCmd::begin(options::GraphicsPrimitive::Rects),
            DLCmd::from_raw(0x1f000009),
        );
        assert_eq!(
            DLCmd::bitmap_ext_format(options::BitmapExtFormat::ARGB1555),
            DLCmd::from_raw(0x2e000000),
        );
        assert_eq!(
            DLCmd::bitmap_ext_format(options::BitmapExtFormat::ARGB4),
            DLCmd::from_raw(0x2e000006),
        );
        assert_eq!(
            DLCmd::bitmap_ext_format(options::BitmapExtFormat::TextVGA),
            DLCmd::from_raw(0x2e00000a),
        );
        assert_eq!(
            DLCmd::bitmap_handle(options::BitmapHandle::force_raw(0)),
            DLCmd::from_raw(0x05000000),
        );
        assert_eq!(
            DLCmd::bitmap_handle(options::BitmapHandle::force_raw(15)),
            DLCmd::from_raw(0x0500000f),
        );
        assert_eq!(
            DLCmd::bitmap_handle(options::BitmapHandle::force_raw(31)),
            DLCmd::from_raw(0x0500001f),
        );
        assert_eq!(
            DLCmd::bitmap_layout(options::BitmapFormat::ARGB4, 255, 255),
            DLCmd::from_raw(0x0731feff),
        );
        assert_eq!(
            DLCmd::bitmap_layout(options::BitmapFormat::ARGB4, 1024, 768),
            DLCmd::from_raw(0x07300100),
        );
        assert_eq!(
            DLCmd::bitmap_layout_h(255, 255),
            DLCmd::from_raw(0x28000000)
        );
        assert_eq!(
            DLCmd::bitmap_layout_h(1024, 768),
            DLCmd::from_raw(0x28000004)
        );
        assert_eq!(
            DLCmd::bitmap_layout_pair(options::BitmapFormat::ARGB4, 255, 255),
            (DLCmd::from_raw(0x0731feff), DLCmd::from_raw(0x28000000)),
        );
        assert_eq!(
            DLCmd::bitmap_layout_pair(options::BitmapFormat::ARGB4, 1024, 768),
            (DLCmd::from_raw(0x07300100), DLCmd::from_raw(0x28000004)),
        );
        assert_eq!(
            DLCmd::bitmap_size(
                255,
                255,
                options::BitmapSizeFilter::Nearest,
                options::BitmapWrapMode::Border,
                options::BitmapWrapMode::Border
            ),
            DLCmd::from_raw(0x0801feff),
        );
        assert_eq!(
            DLCmd::bitmap_size(
                2048,
                2048,
                options::BitmapSizeFilter::Nearest,
                options::BitmapWrapMode::Border,
                options::BitmapWrapMode::Border
            ),
            DLCmd::from_raw(0x08000000),
        );
        assert_eq!(
            DLCmd::bitmap_size(
                1024,
                768,
                options::BitmapSizeFilter::Nearest,
                options::BitmapWrapMode::Border,
                options::BitmapWrapMode::Border
            ),
            DLCmd::from_raw(0x08000100),
        );
        assert_eq!(
            DLCmd::bitmap_size(
                1,
                1,
                options::BitmapSizeFilter::Bilinear,
                options::BitmapWrapMode::Border,
                options::BitmapWrapMode::Border
            ),
            DLCmd::from_raw(0x08100201),
        );
        assert_eq!(
            DLCmd::bitmap_size(
                1,
                1,
                options::BitmapSizeFilter::Nearest,
                options::BitmapWrapMode::Repeat,
                options::BitmapWrapMode::Border
            ),
            DLCmd::from_raw(0x08080201),
        );
        assert_eq!(
            DLCmd::bitmap_size(
                1,
                1,
                options::BitmapSizeFilter::Nearest,
                options::BitmapWrapMode::Border,
                options::BitmapWrapMode::Repeat
            ),
            DLCmd::from_raw(0x08040201),
        );
        assert_eq!(
            DLCmd::bitmap_size_pair(
                255,
                255,
                options::BitmapSizeFilter::Nearest,
                options::BitmapWrapMode::Border,
                options::BitmapWrapMode::Border
            ),
            (DLCmd::from_raw(0x0801feff), DLCmd::from_raw(0x29000000))
        );
        assert_eq!(
            DLCmd::bitmap_size_pair(
                2048,
                2048,
                options::BitmapSizeFilter::Nearest,
                options::BitmapWrapMode::Border,
                options::BitmapWrapMode::Border
            ),
            (DLCmd::from_raw(0x08000000), DLCmd::from_raw(0x29000000)),
        );
        assert_eq!(
            DLCmd::bitmap_size_pair(
                1024,
                768,
                options::BitmapSizeFilter::Nearest,
                options::BitmapWrapMode::Border,
                options::BitmapWrapMode::Border
            ),
            (DLCmd::from_raw(0x08000100), DLCmd::from_raw(0x29000401)),
        );
    }
}
