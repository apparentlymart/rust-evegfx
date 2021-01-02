//! Types use during EVE chip initialization.
//!
//! The types in this package are used as arguments for some of the methods
//! of [`EVE`](super::EVE).

use crate::interface::Interface;
use crate::models::Model;
use crate::EVE;

/// Selects whether the EVE chip should use its internal oscillator or if
/// it should expect external clock signals.
pub enum ClockSource {
    Internal,
    External,
}

/// Represents the timing parameters for video output.
#[derive(Debug)]
pub struct VideoTimings {
    pub sysclk_freq: ClockFrequency,
    pub pclk_div: u8,
    pub pclk_pol: ClockPolarity,
    pub horiz: VideoTimingDimension,
    pub vert: VideoTimingDimension,
}

/// Represents the period transition cycles for one dimension (horizontal or
/// vertical) of the video raster.
///
/// For horizontal parameters, the values are in pixel clocks. For vertical
/// parameters, the values are in lines.
#[derive(Debug)]
pub struct VideoTimingDimension {
    pub total: u16,
    pub visible: u16,
    pub offset: u16,
    pub sync_start: u16,
    pub sync_end: u16,
}

impl VideoTimings {
    /// Timing settings to approximate what's expected for a 720p signal.
    /// This mode is only available on EVE devices that are able to
    /// switch the system clock to 72MHz, thus allowing this mode to
    /// use a 72MHz pixel clock, which is approximately the 74.25MHz that
    /// 720p nominally requires.
    pub const MODE_720P: Self = Self {
        sysclk_freq: ClockFrequency::F72MHz,
        pclk_div: 1,
        pclk_pol: ClockPolarity::RisingEdge,
        horiz: VideoTimingDimension::calculate(1280, 110, 40, 220),
        vert: VideoTimingDimension::calculate(720, 5, 5, 370),
    };
}

impl VideoTimingDimension {
    /// Calculates a `VideoTimingDimension` from the sizes of the
    /// individual periods in the cycle.
    ///
    /// An `VideoTimingDimension` captures the number of steps _into_ a
    /// period where each event occurs, but when describing a mode we often
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

/// Represents the electrical characteristics of the EVE RGB interface.
///
/// This behaves as a "builder" type, with methods that modify its parameters.
/// The default value for each parameter matches the reset values of the EVE
/// chip itself.
#[derive(Debug, Default)]
pub struct RGBElectricalMode {
    pclk_spread: bool,
    channel_bits: (u8, u8, u8),
    dither: bool,
    // TODO: REG_SWIZZLE
}

impl RGBElectricalMode {
    pub fn new() -> Self {
        core::default::Default::default()
    }

    pub fn pclk_spread<'a>(&'a mut self, v: bool) -> &'a mut Self {
        self.pclk_spread = v;
        self
    }

    pub fn channel_bits<'a>(&'a mut self, r: u8, g: u8, b: u8) -> &'a mut Self {
        self.channel_bits = (r, g, b);
        self
    }

    pub fn dither<'a>(&'a mut self, v: bool) -> &'a mut Self {
        self.dither = v;
        self
    }
}

/// Selects which clock edge of the pixel clock where video data will be sampled.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

/// Selects a clock frequency for the system clock.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

const DIMENSION_MASK: u16 = 0b0000111111111111;

pub(crate) fn activate_system_clock<M: Model, I: Interface>(
    eve: &mut EVE<M, I>,
    source: ClockSource,
    video: &VideoTimings,
) -> Result<(), I::Error> {
    use crate::host_commands::HostCmd::*;

    let ll = &mut eve.ll;

    {
        let ei = ll.borrow_interface();
        ei.reset()?;
    };

    // Just in case the system was already activated before we were
    // called, we'll put it to sleep while we do our work here.
    ll.host_command(PWRDOWN, 0, 0)?;
    ll.host_command(ACTIVE, 0, 0)?;
    ll.host_command(SLEEP, 0, 0)?;

    // Internal or external clock source?
    match source {
        ClockSource::Internal => {
            ll.host_command(CLKINT, 0, 0)?;
        }
        ClockSource::External => {
            ll.host_command(CLKEXT, 0, 0)?;
        }
    }

    // Set the system clock frequency.
    {
        let clksel = video.sysclk_freq.cmd_clksel_args();
        ll.host_command(CLKSEL, clksel.0, clksel.1)?;
    }

    // Activate the system clock.
    ll.host_command(ACTIVE, 0, 0)?;

    // Pulse the reset signal to the rest of the device.
    ll.host_command(RST_PULSE, 0, 0)?;

    Ok(())
}

// Busy-waits until the IC signals that it's ready by responding to the
// ID register. Will poll the number of times given in `poll_limit` before
// giving up and returning `Ok(false)`. Will return `Ok(true)` as soon as
// a poll returns the ready value.
pub(crate) fn poll_for_boot<M: Model, I: Interface>(
    eve: &mut EVE<M, I>,
    poll_limit: u32,
) -> Result<bool, I::Error> {
    use crate::registers::Register::*;
    let ll = &mut eve.ll;
    let mut poll = 0;
    while poll < poll_limit {
        let v = ll.rd8(ll.reg_ptr(ID))?;
        if v == 0x7c {
            break;
        }
        poll += 1;
    }
    while poll < poll_limit {
        let v = ll.rd8(ll.reg_ptr(CPURESET))?;
        if v == 0x00 {
            return Ok(true);
        }
        poll += 1;
    }
    return Ok(false);
}

pub(crate) fn activate_pixel_clock<M: Model, I: Interface>(
    eve: &mut EVE<M, I>,
    c: &VideoTimings,
) -> Result<(), I::Error> {
    use crate::registers::Register::*;
    const DIM_MASK: u16 = DIMENSION_MASK;

    let ll = &mut eve.ll;

    ll.wr32(M::reg_ptr(FREQUENCY), c.sysclk_freq.reg_frequency_value())?;

    ll.wr16(M::reg_ptr(VSYNC0), c.vert.sync_start & DIM_MASK)?;
    ll.wr16(M::reg_ptr(VSYNC1), c.vert.sync_end & DIM_MASK)?;
    ll.wr16(M::reg_ptr(VSIZE), c.vert.visible & DIM_MASK)?;
    ll.wr16(M::reg_ptr(VOFFSET), c.vert.offset & DIM_MASK)?;
    ll.wr16(M::reg_ptr(VCYCLE), c.vert.total & DIM_MASK)?;

    ll.wr16(M::reg_ptr(HSYNC0), c.horiz.sync_start & DIM_MASK)?;
    ll.wr16(M::reg_ptr(HSYNC1), c.horiz.sync_end & DIM_MASK)?;
    ll.wr16(M::reg_ptr(HSIZE), c.horiz.visible & DIM_MASK)?;
    ll.wr16(M::reg_ptr(HOFFSET), c.horiz.offset & DIM_MASK)?;
    ll.wr16(M::reg_ptr(HCYCLE), c.horiz.total & DIM_MASK)?;

    ll.wr8(M::reg_ptr(PCLK_POL), c.pclk_pol.reg_pclk_pol_value())?;

    // This one must be last because it actually activates the display.
    ll.wr8(M::reg_ptr(PCLK), c.pclk_div)?;

    Ok(())
}

pub(crate) fn configure_video_pins<M: Model, I: Interface>(
    eve: &mut EVE<M, I>,
    _mode: &RGBElectricalMode,
) -> Result<(), I::Error> {
    // TODO: Actually respect the mode settings. For now, just hard-coded.
    use crate::registers::Register::*;

    let ll = &mut eve.ll;

    ll.wr8(M::reg_ptr(OUTBITS), 0)?;
    ll.wr8(M::reg_ptr(DITHER), 0)?;
    ll.wr8(M::reg_ptr(SWIZZLE), 0)?;
    ll.wr8(M::reg_ptr(CSPREAD), 0)?;
    ll.wr8(M::reg_ptr(ADAPTIVE_FRAMERATE), 0)?;
    ll.wr8(M::reg_ptr(GPIO), 0x83)?;

    Ok(())
}
