use crate::models::Model;
use crate::{Interface, EVE};

pub enum EVEClockSource {
    Internal,
    External,
}

pub(crate) fn activate_system_clock<M: Model, I: Interface>(
    eve: &mut EVE<M, I>,
    source: crate::init::EVEClockSource,
    mode: crate::graphics_mode::EVEGraphicsTimings,
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
        EVEClockSource::Internal => {
            ll.host_command(CLKINT, 0, 0)?;
        }
        EVEClockSource::External => {
            ll.host_command(CLKEXT, 0, 0)?;
        }
    }

    // Set the system clock frequency.
    {
        let clksel = mode.sysclk_freq.cmd_clksel_args();
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
    c: crate::graphics_mode::EVEGraphicsTimings,
) -> Result<(), I::Error> {
    use crate::registers::Register::*;
    const DIM_MASK: u16 = crate::graphics_mode::DIMENSION_MASK;

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
    _mode: crate::graphics_mode::EVERGBElectricalMode,
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
