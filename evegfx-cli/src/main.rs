// In the long run this will hopefully become a convenient CLI command for
// sending/recieving data from an EVE chip via various "normal computer" sorts
// of interfaces (Linux spidev, SPIDriver adapter, etc). For now though it's
// just a small test bed for trying out the library crates in practice.

use evegfx::interface::{EVEAddress, EVECommand};
use evegfx::{EVEInterface, EVE};
use serial_embedded_hal::{PortSettings, Serial};
use spidriver::SPIDriver;
use std::path::Path;

const GAMEDUINO_HDMI_720P: evegfx::graphics_mode::EVEGraphicsTimings =
    evegfx::graphics_mode::EVEGraphicsTimings {
        sysclk_freq: evegfx::graphics_mode::ClockFrequency::F72MHz,
        pclk_div: 1,
        pclk_pol: evegfx::graphics_mode::ClockPolarity::RisingEdge,
        horiz: evegfx::graphics_mode::EVEGraphicsModeDimension {
            total: 1650,
            offset: 260,
            visible: 1280,
            sync_start: 40,
            sync_end: 0,
        },
        vert: evegfx::graphics_mode::EVEGraphicsModeDimension {
            total: 750,
            offset: 25,
            visible: 720,
            sync_start: 5,
            sync_end: 0,
        },
    };

fn main() {
    println!("Hello, world!");

    let serial = Serial::new(
        Path::new("/dev/ttyUSB0"),
        &PortSettings {
            baud_rate: serial_embedded_hal::BaudRate::BaudOther(460800),
            char_size: serial_embedded_hal::CharSize::Bits8,
            parity: serial_embedded_hal::Parity::ParityNone,
            stop_bits: serial_embedded_hal::StopBits::Stop1,
            flow_control: serial_embedded_hal::FlowControl::FlowNone,
        },
    )
    .unwrap();
    let (tx, rx) = serial.split();
    let mut sd = SPIDriver::new(tx, rx);
    sd.unselect().unwrap();
    let eve_interface = evegfx_spidriver::EVESPIDriverInterface::new(sd);
    let mut eve_interface = LogInterface::new(eve_interface);
    //eve_interface.set_fake_delay(std::time::Duration::from_millis(1000));
    eve_interface.clear_fake_delay();

    //let mut ll = evegfx::low_level::EVELowLevel::new(eve_interface);
    /*ll.host_command(evegfx::host_commands::EVEHostCmd::ACTIVE, 0, 0)
        .unwrap();
    ll.host_command(evegfx::host_commands::EVEHostCmd::RST_PULSE, 0, 0)
        .unwrap();
    loop {
        let v = ll.rd16(EVEAddress::force_raw(0x302000)).unwrap();
        println!("Register contains {:#04x}", v);
    }*/
    /*
    show_register(&mut ll, evegfx::registers::EVERegister::FREQUENCY);
    show_register(&mut ll, evegfx::registers::EVERegister::VSYNC0);
    show_register(&mut ll, evegfx::registers::EVERegister::VSYNC1);
    show_register(&mut ll, evegfx::registers::EVERegister::VSIZE);
    show_register(&mut ll, evegfx::registers::EVERegister::VOFFSET);
    show_register(&mut ll, evegfx::registers::EVERegister::VCYCLE);
    show_register(&mut ll, evegfx::registers::EVERegister::HSYNC0);
    show_register(&mut ll, evegfx::registers::EVERegister::HSYNC1);
    show_register(&mut ll, evegfx::registers::EVERegister::HSIZE);
    show_register(&mut ll, evegfx::registers::EVERegister::HOFFSET);
    show_register(&mut ll, evegfx::registers::EVERegister::HCYCLE);
    show_register(&mut ll, evegfx::registers::EVERegister::PCLK_POL);
    show_register(&mut ll, evegfx::registers::EVERegister::PCLK);
    show_register(&mut ll, evegfx::registers::EVERegister::OUTBITS);
    show_register(&mut ll, evegfx::registers::EVERegister::DITHER);
    show_register(&mut ll, evegfx::registers::EVERegister::GPIO);
    show_register(&mut ll, evegfx::registers::EVERegister::CSPREAD);
    show_register(&mut ll, evegfx::registers::EVERegister::ADAPTIVE_FRAMERATE);
    */
    //show_current_dl(&mut ll);
    //return;

    println!("Starting the system clock...");
    let mut eve = EVE::new(eve_interface);
    eve.start_system_clock(evegfx::EVEClockSource::Internal, GAMEDUINO_HDMI_720P)
        .unwrap();
    println!("Waiting for EVE boot...");
    let booted = eve.poll_for_boot(50).unwrap();
    if !booted {
        println!("EVE did not become ready");
        return;
    }

    println!("Configuring video pins...");
    eve.configure_video_pins(evegfx::EVERGBElectricalMode {
        channel_bits: (8, 8, 8),
        dither: false,
        pclk_spread: true,
    })
    .unwrap();
    println!("Sending initial display list...");
    eve.new_display_list(|b| {
        b.clear_color_rgb(evegfx::color::EVEColorRGB {
            r: 255,
            g: 255,
            b: 255,
        })?;
        b.clear_all()?;
        b.display()
    })
    .unwrap();
    eve.new_display_list(|b| {
        b.clear_color_rgb(evegfx::color::EVEColorRGB {
            r: 255,
            g: 255,
            b: 255,
        })?;
        b.clear_all()?;
        b.display()
    })
    .unwrap();
    println!("Activating the pixel clock...");
    eve.start_video(evegfx::EVEGraphicsTimings::MODE_720P)
        .unwrap();
    println!("All done!");

    /*
    let ll = eve.borrow_low_level();
    show_current_dl(ll);
    */
}

fn show_register<I: EVEInterface>(
    ll: &mut evegfx::low_level::EVELowLevel<I>,
    reg: evegfx::registers::EVERegister,
) {
    let v = ll.rd32(reg.into()).unwrap_or(0xf33df4c3);
    println!("Register {:?} contains {:#010x}", reg, v);
}

fn show_mem_rd32<I: EVEInterface>(
    ll: &mut evegfx::low_level::EVELowLevel<I>,
    addr: evegfx::interface::EVEAddress,
) {
    let v = ll.rd32(addr).unwrap_or(0xf33df4c3);
    println!("At {:?} we have {:#010x}", addr, v);
}

fn show_current_dl<I: EVEInterface>(ll: &mut evegfx::low_level::EVELowLevel<I>) {
    let mut offset = 0 as u32;
    let base = evegfx::interface::EVEAddressRegion::RAM_DL.base;
    let length = evegfx::interface::EVEAddressRegion::RAM_DL.length;
    loop {
        if offset >= length {
            return;
        }
        let v = ll.rd32(base + offset).unwrap_or(0xf33df4c3);
        println!("{:#06x}: {:#010x}", offset, v);
        if (v & 0xff000000) == 0 {
            // DISPLAY command ends the display list.
            return;
        }
        offset += 4;
    }
}

/// An `EVEInterface` that wraps another `EVEInterface` and then logs all
/// of the operations on it, for debugging purposes.
struct LogInterface<W: EVEInterface> {
    w: W,
    fake_delay: Option<std::time::Duration>,
}

impl<W: EVEInterface> LogInterface<W> {
    pub fn new(wrapped: W) -> Self {
        Self {
            w: wrapped,
            fake_delay: None,
        }
    }

    pub fn handle<E>(
        result: std::result::Result<(), E>,
        delay: Option<std::time::Duration>,
    ) -> std::result::Result<(), E> {
        if let Some(dur) = delay {
            std::thread::sleep(dur);
        }
        if let Err(err) = result {
            println!("  FAILED!");
            return Err(err);
        }
        result
    }

    pub fn set_fake_delay(&mut self, delay: std::time::Duration) {
        self.fake_delay = Some(delay);
    }

    pub fn clear_fake_delay(&mut self) {
        self.fake_delay = None;
    }
}

impl<W: EVEInterface> EVEInterface for LogInterface<W> {
    type Error = W::Error;

    fn begin_write(&mut self, addr: EVEAddress) -> std::result::Result<(), Self::Error> {
        println!("- begin_write({:?})", addr);
        Self::handle(self.w.begin_write(addr), self.fake_delay)
    }

    fn continue_write(&mut self, v: &[u8]) -> std::result::Result<(), Self::Error> {
        println!("- continue_write({:#x?})", v);
        Self::handle(self.w.continue_write(v), self.fake_delay)
    }

    fn end_write(&mut self) -> std::result::Result<(), Self::Error> {
        println!("- end_write()");
        Self::handle(self.w.end_write(), self.fake_delay)
    }

    fn begin_read(&mut self, addr: EVEAddress) -> std::result::Result<(), Self::Error> {
        println!("- begin_read({:?})", addr);
        Self::handle(self.w.begin_read(addr), self.fake_delay)
    }

    fn continue_read(&mut self, into: &mut [u8]) -> std::result::Result<(), Self::Error> {
        print!("- continue_read(");
        let result = self.w.continue_read(into);
        match result {
            Ok(v) => {
                println!("{:#x?})", into);
                Self::handle(Ok(v), self.fake_delay)
            }
            Err(err) => {
                println!("/* {:?} */)", into.len());
                Self::handle(Err(err), self.fake_delay)
            }
        }
    }

    fn end_read(&mut self) -> std::result::Result<(), Self::Error> {
        println!("- end_read()");
        Self::handle(self.w.end_read(), self.fake_delay)
    }

    fn cmd(&mut self, cmd: EVECommand, a0: u8, a1: u8) -> std::result::Result<(), Self::Error> {
        match evegfx::host_commands::EVEHostCmd::from_interface(cmd) {
            Some(cmd) => println!("- cmd({:?}.into(), {:#04x}, {:#04x})", cmd, a0, a1),
            None => println!("- cmd({:?}, {:#04x}, {:#04x})", cmd, a0, a1),
        }
        Self::handle(self.w.cmd(cmd, a0, a1), self.fake_delay)
    }
}
