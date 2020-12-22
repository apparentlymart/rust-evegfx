// In the long run this will hopefully become a convenient CLI command for
// sending/recieving data from an EVE chip via various "normal computer" sorts
// of interfaces (Linux spidev, SPIDriver adapter, etc). For now though it's
// just a small test bed for trying out the library crates in practice.

use evegfx::display_list::DLCmd;
use evegfx::low_level::EVELowLevel;
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
    let mut eve_ll = EVELowLevel::new(eve_interface);

    for offset in 0..32 {
        let v = eve_ll
            .rd32(evegfx::low_level::RAM_DL + (offset * DLCmd::LENGTH))
            .unwrap();
        println!("At DL {:#x} we have {:#x}", offset, v);
    }

    for offset in 0..32 {
        eve_ll
            .wr8(evegfx::low_level::RAM_G + offset, offset as u8)
            .unwrap();
    }

    for offset in 0..32 {
        let v = eve_ll.rd8(evegfx::low_level::RAM_G + offset).unwrap();
        println!("At G {:#x} we have {:#x}", offset, v);
    }

    let mut buf: [u8; 32] = [0; 32];
    eve_ll.rd8s(evegfx::low_level::RAM_G, &mut buf).unwrap();
    println!("In G we have {:?}", buf);
}
