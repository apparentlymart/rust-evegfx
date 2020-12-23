pub struct EVEGraphicsMode {
    pub timings: EVEGraphicsTimings,
    pub electrical: EVERGBElectricalMode,
}

pub struct EVEGraphicsTimings {
    pub sysclk_freq: ClockFrequency,
    pub pclk_div: u8,
    pub pclk_pol: ClockPolarity,
    pub horiz: EVEGraphicsModeDimension,
    pub vert: EVEGraphicsModeDimension,
}

pub struct EVEGraphicsModeDimension {
    pub total: u16,
    pub visible: u16,
    pub offset: u16,
    pub sync_start: u16,
    pub sync_end: u16,
}

pub struct EVERGBElectricalMode {
    pub pclk_spread: bool,
    pub channel_bits: (u8, u8, u8),
    pub dither: bool,
    // TODO: REG_SWIZZLE
}

impl EVEGraphicsTimings {
    /// Timing settings to approximate what's expected for a 720p signal.
    /// This mode is only available on EVE devices that are able to
    /// switch the system clock to 72MHz, thus allowing this mode to
    /// use a 72MHz pixel clock, which is approximately the 74.25MHz that
    /// 720p nominally requires.Copy
    pub const MODE_720P: Self = Self {
        sysclk_freq: ClockFrequency::F72MHz,
        pclk_div: 1,
        pclk_pol: ClockPolarity::RisingEdge,
        horiz: EVEGraphicsModeDimension {
            total: 1650,
            visible: 1280,
            offset: 370,
            sync_start: 110,
            sync_end: 150,
        },
        vert: EVEGraphicsModeDimension {
            total: 1100,
            visible: 720,
            offset: 380,
            sync_start: 5,
            sync_end: 10,
        },
    };
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ClockPolarity {
    RisingEdge,
    FallingEdge,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ClockFrequency {
    F12MHz,
    F24MHz,
    F36MHz,
    F48MHz,
    F60MHz,
    F72MHz,
}

impl ClockFrequency {
    pub const DEFAULT_SYSCLK_FREQ: Self = Self::F12MHz;

    pub const fn cmd_clksel_a0(self) -> u8 {
        match self {
            ClockFrequency::F12MHz => 0,
            ClockFrequency::F24MHz => 2,
            ClockFrequency::F36MHz => 3,
            ClockFrequency::F48MHz => 4,
            ClockFrequency::F60MHz => 5,
            ClockFrequency::F72MHz => 6,
        }
    }

    pub const fn reg_frequency_value(self) -> u32 {
        ClockFrequency::F12MHz => 0,
        ClockFrequency::F24MHz => 2,
        ClockFrequency::F36MHz => 3,
        ClockFrequency::F48MHz => 4,
        ClockFrequency::F60MHz => 5,
        ClockFrequency::F72MHz => 6,
    }
}

/// Returns `true` if and only if the given value is within the valid range
/// for the fields of `EVEGraphicsModeTimings`. If any of those fields are
/// set to an invalid dimension value then they'll wrap around in the valid
/// range.
pub const fn dimension_is_valid(v: u16) -> bool {
    (v & !DIMENSION_MASK) == 0
}

const DIMENSION_MASK: u16 = 0b0000111111111111;
