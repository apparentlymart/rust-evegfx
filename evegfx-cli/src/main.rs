// In the long run this will hopefully become a convenient CLI command for
// sending/recieving data from an EVE chip via various "normal computer" sorts
// of interfaces (Linux spidev, SPIDriver adapter, etc). For now though it's
// just a small test bed for trying out the library crates in practice.

#![allow(dead_code)]
#![allow(unused_imports)]

use evegfx::config;
use evegfx::display_list::Builder;
use evegfx::interface::Interface;
use evegfx::memory::region::MemoryRegion;
use evegfx::models::Model;
use evegfx::EVE;
use serial_embedded_hal::{PortSettings, Serial};
use spidriver::SPIDriver;
use std::path::Path;

const GAMEDUINO_HDMI_720P: config::VideoTimings = config::VideoTimings {
    sysclk_freq: config::ClockFrequency::F72MHz,
    pclk_div: 1,
    pclk_pol: config::ClockPolarity::RisingEdge,
    horiz: config::VideoTimingDimension {
        total: 1650,
        offset: 260,
        visible: 1280,
        sync_start: 40,
        sync_end: 0,
    },
    vert: config::VideoTimingDimension {
        total: 750,
        offset: 25,
        visible: 720,
        sync_start: 5,
        sync_end: 0,
    },
};

fn main() {
    println!("Hello, world!");

    let ptr: evegfx::memory::Ptr<<evegfx::BT815 as Model>::MainMem> = evegfx::memory::Ptr::new(2);
    println!(
        "message is {:?}",
        evegfx::format!("hello %s %d %x %c", ptr, 4, 6, 'd')
    );

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

    //eve_interface.reset().unwrap();
    //let mut ll = evegfx::low_level::LowLevel::new(eve_interface);
    /*
    ll.host_command(evegfx::host_commands::HostCmd::ACTIVE, 0, 0)
        .unwrap();
    ll.host_command(evegfx::host_commands::HostCmd::RST_PULSE, 0, 0)
        .unwrap();
    */
    /*
    loop {
        let v = ll.rd16(EVEAddress::force_raw(0x302000)).unwrap();
        println!("Register contains {:#04x}", v);
    }
    */
    /*
    show_register(&mut ll, evegfx::registers::Register::FREQUENCY);
    show_register(&mut ll, evegfx::registers::Register::VSYNC0);
    show_register(&mut ll, evegfx::registers::Register::VSYNC1);
    show_register(&mut ll, evegfx::registers::Register::VSIZE);
    show_register(&mut ll, evegfx::registers::Register::VOFFSET);
    show_register(&mut ll, evegfx::registers::Register::VCYCLE);
    show_register(&mut ll, evegfx::registers::Register::HSYNC0);
    show_register(&mut ll, evegfx::registers::Register::HSYNC1);
    show_register(&mut ll, evegfx::registers::Register::HSIZE);
    show_register(&mut ll, evegfx::registers::Register::HOFFSET);
    show_register(&mut ll, evegfx::registers::Register::HCYCLE);
    show_register(&mut ll, evegfx::registers::Register::PCLK_POL);
    show_register(&mut ll, evegfx::registers::Register::PCLK);
    show_register(&mut ll, evegfx::registers::Register::OUTBITS);
    show_register(&mut ll, evegfx::registers::Register::DITHER);
    show_register(&mut ll, evegfx::registers::Register::GPIO);
    show_register(&mut ll, evegfx::registers::Register::CSPREAD);
    show_register(&mut ll, evegfx::registers::Register::ADAPTIVE_FRAMERATE);
    */
    //show_current_dl(&mut ll);
    //return;

    const TIMINGS: config::VideoTimings = GAMEDUINO_HDMI_720P;

    //let mut eve = evegfx::BT815::new(eve_interface);
    let mut eve = EVE::new(evegfx::BT815, eve_interface);
    eve.start_system_clock(config::ClockSource::Internal, &TIMINGS)
        .unwrap();
    println!("Waiting for EVE boot...");
    let booted = eve.poll_for_boot(50).unwrap();
    if !booted {
        println!("EVE did not become ready");
        return;
    }

    println!("Configuring video pins...");
    eve.configure_video_pins(
        config::RGBElectricalMode::new()
            .channel_bits(8, 8, 8)
            .pclk_spread(true),
    )
    .unwrap();
    println!("Sending initial display list...");
    eve.new_display_list(|b| {
        b.clear_color_rgb(evegfx::graphics::RGB {
            r: 255,
            g: 255,
            b: 255,
        })?;
        b.clear_all()?;
        b.display()
    })
    .unwrap();
    eve.new_display_list(|b| {
        use evegfx::display_list::options::*;
        use evegfx::display_list::Builder;
        use evegfx::graphics::*;
        b.clear_color_rgb(RGB { r: 0, g: 0, b: 10 })?;
        b.clear_all()?;
        b.begin(GraphicsPrimitive::Points)?;
        b.point_size(100)?;
        b.vertex_2f((1000, 1000))?;
        b.vertex_2f((2000, 2000))?;
        b.vertex_2f((3000, 3000))?;
        b.vertex_2f((4000, 4000))?;
        b.display()
    })
    .unwrap();
    println!("Activating the pixel clock...");
    eve.start_video(&TIMINGS).unwrap();

    println!("Starting coprocessor...");
    let cp = eve.coprocessor_polling().unwrap();
    let mut cp = cp.with_new_waiter(|old| LogWaiter::new(old));

    //println!("Using the coprocessor to show a testcard...");
    //must(cp.start_display_list());
    //must(cp.start_spinner());
    //must(cp.show_manufacturer_logo());

    println!("Using the coprocessor to present a new display list...");
    cp.new_display_list(|cp| {
        use evegfx::commands::options;
        use evegfx::commands::options::Options;
        use evegfx::graphics::*;
        cp.clear_color_rgb(evegfx::graphics::RGB { r: 0, g: 127, b: 0 })?;
        cp.clear_all()?;
        cp.draw_button(
            (10, 10, 200, 100),
            evegfx::format!("Hello!"),
            options::FontRef::new_raw(23),
            options::Button::new().style(options::WidgetStyle::ThreeD),
        )?;
        cp.draw_text(
            (100, 140),
            evegfx::format!("hello %d!", 5),
            options::FontRef::new_raw(18),
            options::Text::new(),
        )?;
        cp.draw_text(
            (100, 200),
            evegfx::format!("hello %d!", 5),
            options::FontRef::new_raw(25),
            options::Text::new(),
        )?;
        cp.display()
    })
    .unwrap();

    println!("Entering main loop...");
    let mut ball_x: i16 = 1000;
    let mut ball_y: i16 = 1000;
    let mut ball_dx: i16 = 40;
    let mut ball_dy: i16 = 40;
    const MAX_X: i16 = 600 * 16;
    const MAX_Y: i16 = 720 * 16;
    {
        println!("Configure bouncing ball display list");
        {
            println!("Set REG_MACRO_1");
            let r = cp.write_register(
                evegfx::low_level::Register::MACRO_1,
                evegfx::display_list::DLCmd::vertex_2f((0, 0)).as_raw(),
            );
            //let r = cp.block_until_video_scanout();
            unwrap_cp(&mut cp, r);
        }
        {
            println!("New display list");
            let r = cp.new_display_list(|cp| {
                println!("clear_color_rgb");
                cp.clear_color_rgb(evegfx::graphics::RGB { r: 0, g: 0, b: 127 })?;
                println!("clear_all");
                cp.clear_all()?;
                println!("begin points");
                cp.begin(evegfx::display_list::options::GraphicsPrimitive::Points)?;
                println!("point size 100");
                cp.point_size(100)?;
                println!("insert macro 1 (our vertex)");
                cp.command_from_macro(1)?;
                println!("display");
                cp.display()
            });
            unwrap_cp(&mut cp, r);
        }
    }
    loop {
        {
            println!("Await video scanout");
            let r = cp.wait_video_scanout();
            //let r = cp.block_until_video_scanout();
            unwrap_cp(&mut cp, r);
        }
        {
            println!("Update vertex macro");
            let r = cp.write_register(
                evegfx::low_level::Register::MACRO_1,
                evegfx::display_list::DLCmd::vertex_2f((ball_x, ball_y)).as_raw(),
            );
            unwrap_cp(&mut cp, r);
        }
        println!("Done with EVE for this frame");
        ball_x += ball_dx;
        ball_y += ball_dy;
        if ball_x < 0 {
            ball_dx = -ball_dx;
            ball_x = -ball_x;
        }
        if ball_y < 0 {
            ball_dy = -ball_dy;
            ball_y = -ball_y;
        }
        if ball_x >= MAX_X {
            ball_dx = -ball_dx;
            ball_x = MAX_X - (ball_x - MAX_X);
        }
        if ball_y >= MAX_Y {
            ball_dy = -ball_dy;
            ball_y = MAX_Y - (ball_y - MAX_Y);
        }
    }

    println!("Waiting for the coprocessor to become idle...");
    {
        let r = cp.block_until_idle();
        unwrap_cp(&mut cp, r);
    }
    cp.block_until_idle().unwrap();

    println!("All done!");

    /*
    let ll = eve.borrow_low_level();
    show_current_dl(ll);
    */
}

extern crate serial_core;

fn unwrap_cp<R>(
    cp: &mut evegfx::commands::Coprocessor<
        evegfx::BT815,
        LogInterface<
            evegfx_spidriver::EVESPIDriverInterface<
                serial_embedded_hal::Tx,
                serial_embedded_hal::Rx,
            >,
        >,
        LogWaiter<
            evegfx::BT815,
            LogInterface<
                evegfx_spidriver::EVESPIDriverInterface<
                    serial_embedded_hal::Tx,
                    serial_embedded_hal::Rx,
                >,
            >,
            evegfx::commands::waiter::PollingWaiter<
                evegfx::BT815,
                LogInterface<
                    evegfx_spidriver::EVESPIDriverInterface<
                        serial_embedded_hal::Tx,
                        serial_embedded_hal::Rx,
                    >,
                >,
            >,
        >,
    >,
    r: Result<
        R,
        evegfx::commands::Error<
            spidriver::Error<serial_core::Error, serial_core::Error>,
            spidriver::Error<serial_core::Error, serial_core::Error>,
        >,
    >,
) -> R {
    match r {
        Ok(r) => r,
        Err(err) => match err {
            evegfx::commands::Error::Interface(err) => {
                panic!("interface error: {:?}", err);
            }
            evegfx::commands::Error::Waiter(err) => {
                panic!("waiter error: {:?}", err);
            }
            evegfx::commands::Error::Fault => {
                println!("Fetching the coprocessor fault message...");
                let fault_msg_raw = cp.coprocessor_fault_msg().unwrap();
                let fault_msg = std::str::from_utf8(fault_msg_raw.as_bytes()).unwrap();
                panic!("coprocessor fault: {:?}", fault_msg);
            }
            _ => {
                panic!("unknown error");
            }
        },
    }
}

fn show_register<M: Model, I: Interface>(
    ll: &mut evegfx::low_level::LowLevel<M, I>,
    reg: evegfx::low_level::Register,
) {
    let v = ll.rd32(M::reg_ptr(reg)).unwrap_or(0xf33df4c3);
    println!("Register {:?} contains {:#010x}", reg, v);
}

fn show_current_dl<M: Model, I: Interface>(ll: &mut evegfx::low_level::LowLevel<M, I>) {
    let mut offset = 0 as u32;
    let length = M::DisplayListMem::LENGTH;
    loop {
        if offset >= length {
            return;
        }
        let v = ll
            .rd32(M::DisplayListMem::ptr(offset))
            .unwrap_or(0xf33df4c3);
        println!("{:#06x}: {:#010x}", offset, v);
        if (v & 0xff000000) == 0 {
            // DISPLAY command ends the display list.
            return;
        }
        offset += 4;
    }
}

/// An `Interface` that wraps another `Interface` and then logs all
/// of the operations on it, for debugging purposes.
struct LogInterface<W: Interface> {
    w: W,
    fake_delay: Option<std::time::Duration>,
}

impl<W: Interface> LogInterface<W> {
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

impl<W: Interface> Interface for LogInterface<W> {
    type Error = W::Error;

    fn reset(&mut self) -> std::result::Result<(), Self::Error> {
        println!("- reset()");
        Self::handle(self.w.reset(), self.fake_delay)
    }

    fn begin_write(&mut self, addr: u32) -> std::result::Result<(), Self::Error> {
        println!("- begin_write({:#08x?})", addr);
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

    fn begin_read(&mut self, addr: u32) -> std::result::Result<(), Self::Error> {
        println!("- begin_read({:#08x?})", addr);
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

    fn host_cmd(&mut self, cmd: u8, a0: u8, a1: u8) -> std::result::Result<(), Self::Error> {
        match evegfx::low_level::HostCmd::from_raw(cmd) {
            Some(cmd) => println!(
                "- cmd(/*{:#04x?}*/ {:?}.into(), {:#04x}, {:#04x})",
                cmd.to_raw(),
                cmd,
                a0,
                a1
            ),
            None => println!("- cmd({:?}, {:#04x}, {:#04x})", cmd, a0, a1),
        }
        Self::handle(self.w.host_cmd(cmd, a0, a1), self.fake_delay)
    }
}

struct LogWaiter<M: Model, I: Interface, W: evegfx::commands::waiter::Waiter<M, I>> {
    w: W,
    _ei: core::marker::PhantomData<I>,
    _m: core::marker::PhantomData<M>,
}

impl<M: Model, I: Interface, W: evegfx::commands::waiter::Waiter<M, I>> LogWaiter<M, I, W> {
    fn new(wrapped: W) -> Self {
        Self {
            w: wrapped,
            _ei: core::marker::PhantomData,
            _m: core::marker::PhantomData,
        }
    }
}

impl<M: Model, I: Interface, W: evegfx::commands::waiter::Waiter<M, I>>
    evegfx::commands::waiter::Waiter<M, I> for LogWaiter<M, I, W>
{
    type Error = W::Error;

    fn wait_for_space(
        &mut self,
        ll: &mut evegfx::low_level::LowLevel<M, I>,
        need: u16,
    ) -> std::result::Result<u16, evegfx::commands::waiter::WaiterError<W::Error>> {
        println!(
            "- waiting for coprocessor buffer to have {} ({:#06x?}) bytes of space",
            need, need,
        );
        let result = self.w.wait_for_space(ll, need);
        match &result {
            Ok(new_space) => {
                println!(
                    "- coprocessor buffer now has {} ({:#06x?}) bytes of space",
                    new_space, new_space
                );
            }
            Err(_) => {
                println!("- failed while waiting for buffer space");
            }
        }
        result
    }
}
