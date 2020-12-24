use crate::{EVEInterface, EVE};

pub enum EVEClockSource {
    Internal,
    External,
}

pub(crate) fn activate_system_clock<I: EVEInterface>(
    eve: &mut EVE<I>,
    source: crate::init::EVEClockSource,
    mode: crate::graphics_mode::EVEGraphicsTimings,
) -> Result<(), I::Error> {
    use crate::host_commands::EVEHostCmd::*;

    let ll = &mut eve.ll;

    // Just in case the system was already activated before we were
    // called, we'll put it to sleep while we do our work here.
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
// ID register. If there is no EVE connected, or if it fails to boot for
// some reason, this will busy-wait forever.
pub(crate) fn poll_for_boot<I: EVEInterface>(eve: &mut EVE<I>) -> Result<(), I::Error> {
    use crate::registers::EVERegister::*;
    let ll = &mut eve.ll;
    loop {
        let v = ll.rd8(ID.into())?;
        if v == 0x7c {
            return Ok(());
        }
    }
}

pub(crate) fn activate_pixel_clock<I: EVEInterface>(
    eve: &mut EVE<I>,
    c: crate::graphics_mode::EVEGraphicsTimings,
) -> Result<(), I::Error> {
    use crate::registers::EVERegister::*;
    const DIM_MASK: u16 = crate::graphics_mode::DIMENSION_MASK;

    let ll = &mut eve.ll;

    ll.wr16(VSYNC0.into(), c.vert.sync_start & DIM_MASK)?;
    ll.wr16(VSYNC1.into(), c.vert.sync_end & DIM_MASK)?;
    ll.wr16(VSIZE.into(), c.vert.visible & DIM_MASK)?;
    ll.wr16(VOFFSET.into(), c.vert.offset & DIM_MASK)?;
    ll.wr16(VCYCLE.into(), c.vert.total & DIM_MASK)?;

    ll.wr16(HSYNC0.into(), c.horiz.sync_start & DIM_MASK)?;
    ll.wr16(HSYNC1.into(), c.horiz.sync_end & DIM_MASK)?;
    ll.wr16(HSIZE.into(), c.horiz.visible & DIM_MASK)?;
    ll.wr16(HOFFSET.into(), c.horiz.offset & DIM_MASK)?;
    ll.wr16(HCYCLE.into(), c.horiz.total & DIM_MASK)?;

    ll.wr8(PCLK_POL.into(), c.pclk_pol.reg_pclk_pol_value())?;

    // This one must be last because it actually activates the display.
    ll.wr8(PCLK.into(), c.pclk_div)?;

    Ok(())
}

pub(crate) fn configure_video_pins<I: EVEInterface>(
    eve: &mut EVE<I>,
    _mode: crate::graphics_mode::EVERGBElectricalMode,
) -> Result<(), I::Error> {
    // TODO: Actually respect the mode settings. For now, just hard-coded.
    use crate::registers::EVERegister::*;

    let ll = &mut eve.ll;

    ll.wr8(SWIZZLE.into(), 0)?;
    ll.wr8(PCLK_POL.into(), 1)?;
    ll.wr8(CSPREAD.into(), 1)?;

    Ok(())
}
