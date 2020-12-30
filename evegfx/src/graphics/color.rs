#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RGBA {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RGB {
    pub const fn as_rgba(self) -> RGBA {
        RGBA {
            r: self.r,
            g: self.g,
            b: self.b,
            a: 0xff,
        }
    }
}

impl RGBA {
    pub const fn as_rgb(self) -> RGB {
        RGB {
            r: self.r,
            g: self.g,
            b: self.b,
        }
    }
}

impl From<RGBA> for RGB {
    fn from(src: RGBA) -> Self {
        src.as_rgb()
    }
}

impl From<RGB> for RGBA {
    fn from(src: RGB) -> Self {
        src.as_rgba()
    }
}
