/// Represents a bitmap stored in the EVE device's memory space, capturing
/// both the physical location of the bitmap (and optional palette) and
/// the pixel format stored there.
///
/// `Bitmap` is parameterized by `MemoryRegion` so it can in principle
/// represent a bitmap in any part of memory, but in practice functions which
/// take bitmap arguments will typically constrain which memory spaces are
/// allowed, to reflect the addressing constraints of the underlying hardware.
pub struct Bitmap<MR: crate::memory::MemoryRegion> {
    pub image_data: crate::memory::Ptr<MR>,
    pub palette_data: Option<crate::memory::Ptr<MR>>,
    pub format: crate::display_list::options::BitmapExtFormat,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

impl<MR: crate::memory::MemoryRegion> Bitmap<MR> {
    /// Constructs a new `Bitmap` with the given dimensions and pixel format,
    /// with no associated palette.
    ///
    /// The stride will be calculated automatically as the smallest stride
    /// possible for the given width in the given pixel format.
    ///
    /// This function only supports the base set of pixel formats, not the
    /// extended formats. If you pass `BitmapFormat::GLFormat` then this
    /// function will panic.
    pub fn new_without_palette(
        format: crate::display_list::options::BitmapFormat,
        base: crate::memory::Ptr<MR>,
        width: u32,
        height: u32,
    ) -> Self {
        use core::convert::TryFrom;
        let ext_format = match crate::display_list::options::BitmapExtFormat::try_from(format) {
            Ok(fmt) => fmt,
            Err(_) => panic!("new_without_palette does not support extended bitmap formats"),
        };
        Self {
            image_data: base,
            palette_data: None,
            format: ext_format,
            width: width,
            height: height,
            stride: format.minimum_stride(width),
        }
    }

    /// Returns the length of the span of bytes representing the pixels in
    /// graphics memory.
    ///
    /// (This doesn't include the palette data, if any.)
    pub fn byte_length(&mut self) -> u32 {
        return self.height * self.stride;
    }

    // Returns a slice representation of the block of memory representing
    // the pixels in graphics memory.
    ///
    /// (This doesn't include the palette data, if any.)
    pub fn data_slice(&mut self) -> crate::memory::Slice<MR> {
        return self.image_data.slice_length(self.byte_length());
    }
}
