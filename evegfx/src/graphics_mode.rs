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
        horiz: EVEGraphicsModeDimension::calculate(1280, 110, 40, 220),
        vert: EVEGraphicsModeDimension::calculate(720, 5, 5, 370),
    };
}

impl EVEGraphicsModeDimension {
    /// Calculates an `EVEGraphicsModeDimension` from the sizes of the
    /// individual periods in the cycle.
    ///
    /// An EVEGraphicsModeDimension captures the number of cycles _into_ a
    /// cycle where each event occurs, but when describing a mode we often
    /// speak of how many cycles each period has on its own, and so this
    /// function allows converting from the latter to the former automatically.
    pub const fn calculate(active: u16, front_porch: u16, sync: u16, back_porch: u16) -> Self {
        Self {
            total: (active + front_porch + sync + back_porch) & DIMENSION_MASK,
            visible: (active) & DIMENSION_MASK,
            offset: (front_porch + sync + back_porch) & DIMENSION_MASK,
            sync_start: (front_porch) & DIMENSION_MASK,
            sync_end: (front_porch + sync) & DIMENSION_MASK,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ClockPolarity {
    RisingEdge,
    FallingEdge,
}

impl ClockPolarity {
    pub const fn reg_pclk_pol_value(self) -> u8 {
        match self {
            Self::RisingEdge => 0,
            Self::FallingEdge => 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ClockFrequency {
    F24MHz,
    F36MHz,
    F48MHz,
    F60MHz,
    F72MHz,
}

impl ClockFrequency {
    pub const DEFAULT_SYSCLK_FREQ: Self = Self::F60MHz;

    pub const fn cmd_clksel_args(self) -> (u8, u8) {
        match self {
            ClockFrequency::F24MHz => (2, 0),
            ClockFrequency::F36MHz => (3, 0),
            ClockFrequency::F48MHz => (4, 0),
            ClockFrequency::F60MHz => (0, 0), // zero for back-compat with older EVE devices
            ClockFrequency::F72MHz => (6, 0),
        }
    }

    pub const fn reg_frequency_value(self) -> u32 {
        match self {
            ClockFrequency::F24MHz => 24000000,
            ClockFrequency::F36MHz => 36000000,
            ClockFrequency::F48MHz => 48000000,
            ClockFrequency::F60MHz => 60000000,
            ClockFrequency::F72MHz => 72000000,
        }
    }
}

/// Returns `true` if and only if the given value is within the valid range
/// for the fields of `EVEGraphicsModeTimings`. If any of those fields are
/// set to an invalid dimension value then they'll wrap around in the valid
/// range.
pub const fn dimension_is_valid(v: u16) -> bool {
    (v & !DIMENSION_MASK) == 0
}

pub const DIMENSION_MASK: u16 = 0b0000111111111111;
