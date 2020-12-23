#[derive(Clone, Copy, PartialEq, Eq)]
pub struct EVEColorRGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct EVEColorRGBA {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl EVEColorRGB {
    pub const fn as_rgba(self) -> EVEColorRGBA {
        EVEColorRGBA {
            r: self.r,
            g: self.g,
            b: self.b,
            a: 0xff,
        }
    }
}

impl EVEColorRGBA {
    pub const fn as_rgb(self) -> EVEColorRGB {
        EVEColorRGB {
            r: self.r,
            g: self.g,
            b: self.b,
        }
    }
}

impl From<EVEColorRGBA> for EVEColorRGB {
    fn from(src: EVEColorRGBA) -> Self {
        src.as_rgb()
    }
}

impl From<EVEColorRGB> for EVEColorRGBA {
    fn from(src: EVEColorRGB) -> Self {
        src.as_rgba()
    }
}
