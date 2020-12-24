// In the long run this will hopefully become a convenient CLI command for
// sending/recieving data from an EVE chip via various "normal computer" sorts
// of interfaces (Linux spidev, SPIDriver adapter, etc). For now though it's
// just a small test bed for trying out the library crates in practice.

use evegfx::EVE;
use serial_embedded_hal::{PortSettings, Serial};
use spidriver::SPIDriver;
use std::path::Path;

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

    println!("Starting the system clock...");
    let mut eve = EVE::new(eve_interface);
    eve.start_system_clock(
        evegfx::EVEClockSource::Internal,
        evegfx::EVEGraphicsTimings::MODE_720P,
    )
    .unwrap();
    println!("Waiting for EVE boot...");
    eve.poll_for_boot().unwrap();

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
    println!("Activating the pixel clock...");
    eve.start_video(evegfx::EVEGraphicsTimings::MODE_720P)
        .unwrap();
    println!("All done!");
}
