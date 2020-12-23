// In the long run this will hopefully become a convenient CLI command for
// sending/recieving data from an EVE chip via various "normal computer" sorts
// of interfaces (Linux spidev, SPIDriver adapter, etc). For now though it's
// just a small test bed for trying out the library crates in practice.

use evegfx::display_list::DLCmd;
use evegfx::host_commands::EVEHostCmd;
use evegfx::low_level::EVELowLevel;
use evegfx::registers::EVERegister;
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
    let mut eve_interface = evegfx_spidriver::EVESPIDriverInterface::new(sd);
    let id_data = evegfx::interface::read_chip_id(&mut eve_interface).unwrap();
    println!(
        "Chip ID data is [{:#04x}, {:#04x}, {:#04x}, {:#04x}]",
        id_data[0], id_data[1], id_data[2], id_data[3]
    );

    let mut eve_ll = EVELowLevel::new(eve_interface);

    eve_ll.host_command(EVEHostCmd::CLKEXT, 0, 0).unwrap();
    eve_ll.host_command(EVEHostCmd::ACTIVE, 0, 0).unwrap();
    eve_ll.wr16(EVERegister::HCYCLE.into(), 548).unwrap();
    eve_ll.wr16(EVERegister::HOFFSET.into(), 43).unwrap();
    eve_ll.wr16(EVERegister::HSYNC0.into(), 0).unwrap();
    eve_ll.wr16(EVERegister::HSYNC1.into(), 41).unwrap();
    eve_ll.wr16(EVERegister::VCYCLE.into(), 292).unwrap();
    eve_ll.wr16(EVERegister::VOFFSET.into(), 12).unwrap();
    eve_ll.wr16(EVERegister::VSYNC0.into(), 0).unwrap();
    eve_ll.wr16(EVERegister::VSYNC1.into(), 10).unwrap();
    eve_ll.wr16(EVERegister::SWIZZLE.into(), 0).unwrap();
    eve_ll.wr16(EVERegister::PCLK_POL.into(), 1).unwrap();
    eve_ll.wr16(EVERegister::CSPREAD.into(), 1).unwrap();
    eve_ll.wr16(EVERegister::HSIZE.into(), 480).unwrap();
    eve_ll.wr16(EVERegister::VSIZE.into(), 272).unwrap();

    for offset in 0..32 {
        let v = eve_ll
            .rd32(evegfx::interface::EVEAddressRegion::RAM_DL + (offset * DLCmd::LENGTH))
            .unwrap();
        println!("At DL {:#x} we have {:#x}", offset, v);
    }

    for offset in 0..32 {
        eve_ll
            .wr8(
                evegfx::interface::EVEAddressRegion::RAM_G + offset,
                offset as u8,
            )
            .unwrap();
    }

    for offset in 0..32 {
        let v = eve_ll
            .rd8(evegfx::interface::EVEAddressRegion::RAM_G + offset)
            .unwrap();
        println!("At G {:#x} we have {:#x}", offset, v);
    }

    let mut buf: [u8; 32] = [0; 32];
    eve_ll
        .rd8s(evegfx::interface::EVEAddressRegion::RAM_G + 0, &mut buf)
        .unwrap();
    println!("In G we have {:?}", buf);
}
