//! Various types used as arguments to coprocessor commands.

pub trait Options: Clone + Copy + PartialEq + Eq {
    fn new() -> Self;
}

pub fn defaults<T: Options>() -> T {
    T::new()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VideoPlayback(u32);

impl Options for VideoPlayback {
    fn new() -> Self {
        Self(0)
    }
}

impl VideoPlayback {
    pub const fn no_tear(self) -> Self {
        Self(self.0 | OPT_NOTEAR)
    }

    pub const fn fullscreen(self) -> Self {
        Self(self.0 | OPT_FULLSCREEN)
    }

    pub const fn decode_audio(self) -> Self {
        Self(self.0 | OPT_SOUND)
    }

    pub fn to_raw(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LoadImage(u32);

impl Options for LoadImage {
    fn new() -> Self {
        Self(0)
    }
}

impl LoadImage {
    pub const fn jpeg_color_mode(self, mode: JPEGColorMode) -> Self {
        Self((self.0 & (!0b1)) | mode as u32)
    }

    pub const fn no_display_list(self) -> Self {
        Self(self.0 | OPT_NODL)
    }

    pub const fn scale_to_screen(self) -> Self {
        Self(self.0 | OPT_FULLSCREEN)
    }

    pub fn to_raw(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Button(u32);

impl Options for Button {
    fn new() -> Self {
        Self(0)
    }
}

impl Button {
    pub const fn style(self, style: WidgetStyle) -> Self {
        const MASK: u32 = !256;
        Self((self.0 & MASK) | style as u32)
    }

    pub fn to_raw(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Text(u32);

impl Options for Text {
    fn new() -> Self {
        Self(0)
    }
}

impl Text {
    pub fn to_raw(self) -> u32 {
        self.0
    }
}

/// Rendering style (flat or 3D) for various widgets that can support these
/// two rendering styles.
#[repr(u32)]
pub enum WidgetStyle {
    Flat = 256,
    ThreeD = 0,
}

#[repr(u32)]
pub enum JPEGColorMode {
    RGB565 = 0,
    Monochrome = 1,
}

/// A reference to a font previously registered in the coprocessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontRef(u8);

impl FontRef {
    const MASK: u8 = 0b00011111;

    /// Takes the given value modulo 32 and uses it to construct a font
    /// reference.
    pub fn new_raw(v: u8) -> Self {
        Self(v & Self::MASK)
    }

    /// Returns the raw representation of the font reference index. Although
    /// returned as a `u8`, the value is always less than 32.
    pub fn to_raw(self) -> u8 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_playback() {
        assert_eq!(VideoPlayback::new().0, 0b00000000000000000000000000000000);
        assert_eq!(
            VideoPlayback::new().fullscreen().0,
            0b00000000000000000000000000001000
        );
        assert_eq!(
            VideoPlayback::new().no_tear().0,
            0b00000000000000000000000000000100
        );
        assert_eq!(
            VideoPlayback::new().decode_audio().0,
            0b00000000000000000000000000100000
        );
        assert_eq!(
            VideoPlayback::new().decode_audio().fullscreen().0,
            0b00000000000000000000000000101000
        );
    }
}

const OPT_NODL: u32 = 2;
const OPT_NOTEAR: u32 = 4;
const OPT_FULLSCREEN: u32 = 8;
const OPT_SOUND: u32 = 32;
