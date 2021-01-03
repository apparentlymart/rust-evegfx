//! Interface to the EVE graphics coprocessor.
//!
//! You can get a [`Coprocessor`] object by calling
//! [`EVE::coprocessor`](crate::EVE::coprocessor) or
//! [`EVE::coprocessor_polling`](crate::EVE::coprocessor_polling)
//! on an [`EVE`](crate::EVE) object for the appropriate EVE model.
//!
//! [`Coprocessor`] is the main interface to the graphics coprocessor.
//! Applications typically switch to primarily using the graphics coprocessor
//! once they've completed initialization, because it allows streaming commands
//! to the EVE chip with the possibility for synchronization with the display
//! raster, and provides higher-level helpers for building display lists.
//!
//! ```rust
//! # let r = evegfx::interface::fake::interface_example(|mut ei| {
//! // "ei" is an implementation of evegfx::interface::Interface.
//! let eve = evegfx::EVE::new(evegfx::BT815, ei);
//! // (...do initial boot sequence for the "eve" object...)
//! # let f = || -> Result<(), evegfx::CoprocessorError<_, _, _>> {
//! let mut cp = eve.coprocessor_polling()?;
//! cp.new_display_list(|cp| {
//!     use evegfx::display_list::Builder; // so trait methods are available
//!     use evegfx::display_list::options;
//!     cp.clear_all()?;
//!     cp.begin(options::GraphicsPrimitive::LineStrip);
//!     cp.draw(options::GraphicsPrimitive::LineStrip, |mut shape| {
//!         shape.vertex_2f((10, 10))?;
//!         shape.vertex_2f((100, 10))?;
//!         shape.vertex_2f((100, 100))?;
//!         shape.vertex_2f((10, 100))?;
//!         shape.vertex_2f((10, 10))
//!     })?;
//!     cp.display()
//! })?;
//! # Ok(()) };
//! # f();
//! # });
//! ```

pub(crate) mod coprocessor;
pub mod options;
pub mod strfmt;
pub mod waiter;

mod command_word;

#[doc(inline)]
pub use coprocessor::{Coprocessor, Error, Result};

#[cfg(test)]
mod tests {
    extern crate std;

    use super::waiter::PollingWaiter;
    use super::*;
    use crate::interface::Interface;
    use crate::memory::MemoryRegion;
    use crate::models::testing::Exhaustive;
    use crate::models::Model;
    use std::vec;
    use std::vec::Vec;

    type MockResult<T> =
        Result<T, Exhaustive, MockInterface, PollingWaiter<Exhaustive, MockInterface>>;
    type MockCoprocessor =
        Coprocessor<Exhaustive, MockInterface, PollingWaiter<Exhaustive, MockInterface>>;

    fn test_obj<F: FnOnce(&mut MockInterface)>(setup: F) -> MockCoprocessor {
        let mut interface = MockInterface::new();
        setup(&mut interface);
        unwrap_copro(Exhaustive::new(interface).coprocessor_polling())
    }

    fn unwrap_copro<R>(v: MockResult<R>) -> R {
        match v {
            Ok(v) => v,
            Err(err) => match err {
                Error::Interface(err) => {
                    std::panic!("interface error: {:?}", err);
                }
                Error::Waiter(_) => {
                    std::panic!("waiter error");
                }
                Error::Fault => {
                    std::panic!("coprocessor fault");
                }
                Error::Unsupported => {
                    std::panic!("unsupported feature");
                }
            },
        }
    }

    #[test]
    fn test_handle_buffer_space() {
        let mut cp = test_obj(|ei| {
            // For this test we'll make the available buffer space much
            // smaller than normal, so we can see the coprocessor object
            // handle what looks like running out of buffer space.
            ei.current_space = 16;
        });

        // Some junk data just to use up buffer space.
        unwrap_copro(cp.append_raw_word(0xf4ce0001));
        unwrap_copro(cp.append_raw_word(0xf4ce0002));
        unwrap_copro(cp.append_raw_word(0xf4ce0003));
        // We now have only four bytes left, but the following command needs
        // eight so we should ReadSpace before writing it.
        unwrap_copro(cp.wait_microseconds(127));

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(16),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xf4ce0001), // Junk data 1
            MockInterfaceCall::Write(0xf4ce0002), // Junk data 2
            MockInterfaceCall::Write(0xf4ce0003), // Junk data 3
            // The coprocessor object now only knows about four remaining
            // bytes of buffer space, but the next command requires eight and
            // so we'll poll to see if more space is available before
            // continuing.
            MockInterfaceCall::StopStream,
            MockInterfaceCall::ReadSpace(16),
            MockInterfaceCall::StartStream,
            // There is now >= 8 bytes buffer space, so we can continue with
            // appending the two words of the wait_microseconds command.
            MockInterfaceCall::Write(0xffffff65), // CMD_WAIT
            MockInterfaceCall::Write(127),        // the duration value from above
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_start_display_list() {
        let mut cp = test_obj(|_| {});

        unwrap_copro(cp.start_display_list());

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xffffff00), // CMD_DLSTART
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_display_list_swap() {
        let mut cp = test_obj(|_| {});

        unwrap_copro(cp.display_list_swap());

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xffffff01), // CMD_SWAP
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_trigger_cmdflag_interrupt() {
        let mut cp = test_obj(|_| {});

        unwrap_copro(cp.trigger_cmdflag_interrupt(core::time::Duration::from_millis(500)));

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xFFFFFF02), // CMD_INTERRUPT
            MockInterfaceCall::Write(500),        // Wait 500ms first
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_cold_start() {
        let mut cp = test_obj(|_| {});

        unwrap_copro(cp.cold_start());

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xFFFFFF32), // CMD_COLDSTART
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_append_display_list_from_main_mem() {
        let mut cp = test_obj(|_| {});

        unwrap_copro(cp.append_display_list_from_main_mem(
            crate::memory::Ptr::new(16)..crate::memory::Ptr::new(24),
        ));

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xFFFFFF1E), // CMD_APPEND
            MockInterfaceCall::Write(16),         // Start address
            MockInterfaceCall::Write(8),          // Number of bytes to copy
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_new_display_list() {
        let mut cp = test_obj(|_| {});

        unwrap_copro(cp.new_display_list(|cp| {
            // We must import this trait in order to call the display list building
            // methods on the coprocessor object.
            use crate::display_list::Builder;

            cp.append_raw_word(0xdeadbeef)?;
            cp.clear_color_alpha(127)
        }));

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xffffff00), // CMD_DLSTART
            MockInterfaceCall::Write(0xdeadbeef), // Fake display list command
            MockInterfaceCall::Write(0x0f00007f), // CLEAR_COLOR_A(127)
            MockInterfaceCall::Write(0xffffff01), // CMD_SWAP
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_show_testcard() {
        let mut cp = test_obj(|_| {});

        unwrap_copro(cp.show_testcard());

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xffffff61), // CMD_TESTCARD
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_show_manufacturer_logo() {
        let mut cp = test_obj(|_| {});

        unwrap_copro(cp.show_manufacturer_logo());

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xffffff31), // CMD_LOGO
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_wait_microseconds() {
        let mut cp = test_obj(|_| {});

        unwrap_copro(cp.wait_microseconds(12345));

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xffffff65), // CMD_WAIT
            MockInterfaceCall::Write(12345),      // the duration value from above
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_write_memory() {
        let mut cp = test_obj(|_| {});

        let ptr = <Exhaustive as crate::models::Model>::MainMem::ptr(16);
        unwrap_copro(cp.write_memory(ptr, b"hello world"));

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xFFFFFF1A), // CMD_MEMWRITE
            MockInterfaceCall::Write(16),         // the target address
            MockInterfaceCall::Write(11),         // the length of the data in bytes
            MockInterfaceCall::Write(0x6c6c6568), // 'h', 'e', 'l', 'l'
            MockInterfaceCall::Write(0x6f77206f), // 'o', ' ', 'w', 'o'
            MockInterfaceCall::Write(0x00646c72), // 'r', 'l', 'd' + '\0' padding byte
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_write_memory_inflate() {
        let mut cp = test_obj(|_| {});

        let ptr = <Exhaustive as crate::models::Model>::MainMem::ptr(21);
        // We don't actually use valid inflate data here because we don't
        // have a real coprocessor under this anyway, but if we sent this
        // data to a real EVE chip then it would generate a fault.
        unwrap_copro(cp.write_memory_inflate(ptr, b"hello world"));

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xFFFFFF22), // CMD_INFLATE
            MockInterfaceCall::Write(21),         // the target address
            // NOTE: Unlike CMD_MEMWRITE there is no explicit length field
            // here, because the deflate stream is self-delimiting and so the
            // coprocessor can tell when it has found the end of it.
            MockInterfaceCall::Write(0x6c6c6568), // 'h', 'e', 'l', 'l'
            MockInterfaceCall::Write(0x6f77206f), // 'o', ' ', 'w', 'o'
            MockInterfaceCall::Write(0x00646c72), // 'r', 'l', 'd' + '\0' padding byte
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_write_memory_image() {
        use options::Options;

        let mut cp = test_obj(|_| {});

        let ptr = <Exhaustive as crate::models::Model>::MainMem::ptr(21);
        // We don't actually use valid image data here because we don't
        // have a real coprocessor under this anyway, but if we sent this
        // data to a real EVE chip then it would generate a fault.
        unwrap_copro(
            cp.write_memory_image(
                ptr,
                b"hello world",
                options::LoadImage::new()
                    .scale_to_screen()
                    .jpeg_color_mode(options::JPEGColorMode::Monochrome),
            ),
        );

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xFFFFFF24), // CMD_LOADIMAGE
            MockInterfaceCall::Write(21),         // the target address
            MockInterfaceCall::Write(8 + 1),      // OPTS_FULLSCREEN + OPT_MONO
            // NOTE: Unlike CMD_MEMWRITE there is no explicit length field
            // here, because the image data is self-delimiting and so the
            // coprocessor can tell when it has found the end of it.
            MockInterfaceCall::Write(0x6c6c6568), // 'h', 'e', 'l', 'l'
            MockInterfaceCall::Write(0x6f77206f), // 'o', ' ', 'w', 'o'
            MockInterfaceCall::Write(0x00646c72), // 'r', 'l', 'd' + '\0' padding byte
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_draw_button_literal() {
        use options::Options as _;
        use strfmt::Message;
        let mut cp = test_obj(|_| {});

        unwrap_copro(cp.draw_button(
            (10, 20, 100, 12),
            Message::new_literal(b"hello world!\0"),
            options::FontRef::new_raw(31),
            options::Button::new().style(options::WidgetStyle::Flat),
        ));

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xffffff0d), // CMD_BUTTON
            MockInterfaceCall::Write(10 | 20 << 16), // the x and y coordinates
            MockInterfaceCall::Write(100 | 12 << 16), // the width and height
            MockInterfaceCall::Write(31 | 256 << 16), // the font index and opts
            MockInterfaceCall::Write(0x6c6c6568), // 'h' 'e' 'l' 'l' (interpreted as LE int)
            MockInterfaceCall::Write(0x6f77206f), // 'o' ' ' 'w' 'o' (interpreted as LE int)
            MockInterfaceCall::Write(0x21646c72), // 'r' 'l' 'd' '!'
            MockInterfaceCall::Write(0x00000000), // null terminator and padding
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_draw_button_fmt() {
        use options::Options as _;
        use strfmt::{Argument, Message};
        let mut cp = test_obj(|_| {});

        unwrap_copro(cp.draw_button(
            (10, 20, 100, 12),
            Message::new(b"hello %x!\0", &[Argument::UInt(0xf33df4c3)]),
            options::FontRef::new_raw(31),
            options::Button::new().style(options::WidgetStyle::Flat),
        ));

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xffffff0d), // CMD_BUTTON
            MockInterfaceCall::Write(10 | 20 << 16), // the x and y coordinates
            MockInterfaceCall::Write(100 | 12 << 16), // the width and height
            MockInterfaceCall::Write(31 | (4096 | 256) << 16), // the font index and opts
            MockInterfaceCall::Write(0x6c6c6568), // 'h' 'e' 'l' 'l' (interpreted as LE int)
            MockInterfaceCall::Write(0x7825206f), // 'o' ' ' '%' 'x' (interpreted as LE int)
            MockInterfaceCall::Write(0x00000021), // '!', null terminator and padding
            MockInterfaceCall::Write(0xf33df4c3), // The format argument
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_block_read_register() {
        let mut cp = test_obj(|ei| {
            // Make sure the function will see enough space to think
            // that the coprocessor is always caught up.
            ei.current_space = 4092;

            // We must fake the REG_CMD_WRITE value for this command, because
            // it expects to find it pointing to the end of the command it
            // just wrote.
            ei.reg_cmd_write_value = 12;

            // We'll return something "interesting" at all other memory
            // addresses, so we can test that this ends up getting returned
            // as the result.
            ei.other_read_value = 0xf33df4c3;
        });

        let result = unwrap_copro(cp.block_read_register(crate::registers::Register::HCYCLE));

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xFFFFFF19), // CMD_REGREAD
            MockInterfaceCall::Write(0x0030202c), // address of REG_HCYCLE
            MockInterfaceCall::Write(0xf0f0f0f0), // placeholder data for result
            MockInterfaceCall::StopStream,
            MockInterfaceCall::ReadWritePtr(12), // Faked pointer to end of command
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::ReadOther(0x00300008, 0xf33df4c3), // Address of the result
            MockInterfaceCall::StartStream,
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);

        // Our mock interface doesn't actually have a coprocessor to write a
        // result into place, so we expect to get back the "other read value"
        // configured above, which is the result of the ReadOther call asserted
        // above.
        debug_assert_eq!(result, 0xf33df4c3);
    }

    #[test]
    fn test_block_for_memory_crc() {
        let mut cp = test_obj(|ei| {
            // Make sure the function will see enough space to think
            // that the coprocessor is always caught up.
            ei.current_space = 4092;

            // We must fake the REG_CMD_WRITE value for this command, because
            // it expects to find it pointing to the end of the command it
            // just wrote.
            ei.reg_cmd_write_value = 12;

            // We'll return something "interesting" at all other memory
            // addresses, so we can test that this ends up getting returned
            // as the result.
            ei.other_read_value = 0xf33df4c3;
        });

        let start_addr = <Exhaustive as Model>::MainMem::ptr(2);
        let end_addr = <Exhaustive as Model>::MainMem::ptr(12);
        let result = unwrap_copro(cp.block_for_memory_crc(start_addr..end_addr));

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xFFFFFF18), // CMD_MEMCRC
            MockInterfaceCall::Write(2),          // start address
            MockInterfaceCall::Write(10),         // data length
            MockInterfaceCall::Write(0xf0f0f0f0), // space for the result to be written
            MockInterfaceCall::StopStream,
            MockInterfaceCall::ReadWritePtr(12), // Faked pointer to end of command
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::ReadOther(0x00300008, 0xf33df4c3), // Address of the result
            MockInterfaceCall::StartStream,
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);

        // Our mock interface doesn't actually have a coprocessor to write a
        // result into place, so we expect to get back the "other read value"
        // configured above, which is the result of the ReadOther call asserted
        // above.
        debug_assert_eq!(result, 0xf33df4c3);
    }

    #[test]
    fn test_use_api_level_1() {
        let mut cp = test_obj(|_| {});

        unwrap_copro(cp.use_api_level_1());

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xFFFFFF63), // CMD_APILEVEL
            MockInterfaceCall::Write(1),          // the version selection
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    #[test]
    fn test_use_api_level_2() {
        let mut cp = test_obj(|_| {});

        unwrap_copro(cp.use_api_level_2());

        let ei = unwrap_copro(cp.take_interface());
        let got = ei.calls();
        let want = vec![
            MockInterfaceCall::ReadSpace(4092),
            MockInterfaceCall::StartStream,
            MockInterfaceCall::Write(0xFFFFFF63), // CMD_APILEVEL
            MockInterfaceCall::Write(2),          // the version selection
            MockInterfaceCall::StopStream,
        ];
        debug_assert_eq!(&got[..], &want[..]);
    }

    /// A test double for `trait Interface`, available only in test mode.
    pub struct MockInterface {
        write_addr: Option<u32>,
        read_addr: Option<u32>,

        pub(crate) current_space: u16,
        pub(crate) reg_cmd_write_value: u32,
        pub(crate) other_read_value: u32,

        // calls_ is the call log. Each call to a mock method appends one
        // entry to this vector, including any that fail.
        calls_: Vec<MockInterfaceCall>,
    }

    #[derive(Clone)]
    pub enum MockInterfaceCall {
        ReadSpace(u16),
        ReadWritePtr(u32),
        ReadOther(u32, u32),
        Write(u32),
        StartStream,
        StopStream,
    }

    impl std::fmt::Debug for MockInterfaceCall {
        fn fmt(
            &self,
            f: &mut core::fmt::Formatter<'_>,
        ) -> core::result::Result<(), core::fmt::Error> {
            match self {
                MockInterfaceCall::ReadSpace(space) => write!(f, "ReadSpace({:#4?})", space),
                MockInterfaceCall::ReadWritePtr(v) => write!(f, "ReadWritePtr({:#010x?})", v),
                MockInterfaceCall::ReadOther(addr, v) => {
                    write!(f, "ReadOther({:#010x?}, {:#x?})", addr, v)
                }
                MockInterfaceCall::Write(v) => write!(f, "Write({:#010x?})", v),
                MockInterfaceCall::StartStream => write!(f, "StartStream"),
                MockInterfaceCall::StopStream => write!(f, "StopStream"),
            }
        }
    }

    impl MockInterface {
        const SPACE_ADDR: u32 = <Exhaustive as Model>::RegisterMem::BASE_ADDR + 0x574;
        const WRITE_ADDR: u32 = <Exhaustive as Model>::RegisterMem::BASE_ADDR + 0x578;
        const WRITTEN_ADDR: u32 = <Exhaustive as Model>::RegisterMem::BASE_ADDR + 0xfc;

        pub fn new() -> Self {
            Self {
                write_addr: None,
                read_addr: None,
                current_space: 0xffc,
                reg_cmd_write_value: 0,
                other_read_value: 0xffffffff,
                calls_: Vec::new(),
            }
        }

        /// Consumes the mock and returns all of the calls it logged
        /// during its life.
        pub fn calls(self) -> Vec<MockInterfaceCall> {
            self.calls_
        }
    }

    #[derive(Debug)]
    pub struct MockError(&'static str);

    impl Interface for MockInterface {
        type Error = MockError;

        fn begin_write(&mut self, addr: u32) -> core::result::Result<(), Self::Error> {
            if let Some(_) = self.write_addr {
                return Err(MockError("begin_write when a write is already active"));
            }
            if let Some(_) = self.read_addr {
                return Err(MockError("begin_write when a read is already active"));
            }
            if addr == Self::WRITE_ADDR {
                self.calls_.push(MockInterfaceCall::StartStream);
            }
            if addr == Self::SPACE_ADDR {
                return Err(MockError("mustn't write to REG_CMDB_SPACE"));
            }
            self.write_addr = Some(addr);
            Ok(())
        }

        fn continue_write(&mut self, buf: &[u8]) -> core::result::Result<(), Self::Error> {
            match self.write_addr {
                Some(addr) => {
                    if addr == Self::WRITE_ADDR {
                        if buf.len() != 4 {
                            return Err(MockError("must write to REG_CMDB_WRITE using wr32"));
                        }
                        let v = (buf[0] as u32)
                            | (buf[1] as u32) << 8
                            | (buf[2] as u32) << 16
                            | (buf[3] as u32) << 24;
                        self.calls_.push(MockInterfaceCall::Write(v));
                    }
                    // We ignore all other writes because they aren't relevant
                    // to our coprocessor testing.
                    Ok(())
                }
                None => Err(MockError("continue_write without an active write")),
            }
        }

        fn end_write(&mut self) -> core::result::Result<(), Self::Error> {
            let result = match self.write_addr {
                Some(addr) => {
                    if addr == Self::WRITE_ADDR {
                        self.calls_.push(MockInterfaceCall::StopStream);
                    }
                    // We ignore all other addresses because they aren't relevant
                    // to our coprocessor testing.
                    Ok(())
                }
                None => Err(MockError("end_write without an active write")),
            };
            self.write_addr = None;
            result
        }

        fn begin_read(&mut self, addr: u32) -> core::result::Result<(), Self::Error> {
            if let Some(_) = self.write_addr {
                return Err(MockError("begin_read when a write is already active"));
            }
            if let Some(_) = self.read_addr {
                return Err(MockError("begin_read when a read is already active"));
            }
            if addr == Self::WRITE_ADDR {
                return Err(MockError("mustn't read from REG_CMDB_WRITE"));
            }
            self.read_addr = Some(addr);
            Ok(())
        }

        fn continue_read(&mut self, into: &mut [u8]) -> core::result::Result<(), Self::Error> {
            match self.read_addr {
                Some(addr) => {
                    match addr {
                        Self::SPACE_ADDR => {
                            if into.len() != 2 {
                                return Err(MockError("must read REG_CMDB_SPACE with rd16"));
                            }
                            self.calls_
                                .push(MockInterfaceCall::ReadSpace(self.current_space));
                            into[0] = (self.current_space & 0xff) as u8;
                            into[1] = (self.current_space >> 8) as u8;
                        }
                        Self::WRITTEN_ADDR => {
                            if into.len() != 4 {
                                return Err(MockError("must read REG_CMD_WRITE with rd32"));
                            }
                            self.calls_
                                .push(MockInterfaceCall::ReadWritePtr(self.reg_cmd_write_value));
                            into[0] = (self.reg_cmd_write_value) as u8;
                            into[1] = (self.reg_cmd_write_value >> 8) as u8;
                            into[2] = (self.reg_cmd_write_value >> 16) as u8;
                            into[3] = (self.reg_cmd_write_value >> 24) as u8;
                        }
                        _ => {
                            match into.len() {
                                1 => {
                                    self.calls_.push(MockInterfaceCall::ReadOther(
                                        addr,
                                        self.other_read_value & 0xff,
                                    ));
                                    into[0] = self.other_read_value as u8;
                                }
                                2 => {
                                    self.calls_.push(MockInterfaceCall::ReadOther(
                                        addr,
                                        self.other_read_value & 0xffff,
                                    ));
                                    into[0] = (self.other_read_value) as u8;
                                    into[1] = (self.other_read_value >> 8) as u8;
                                }
                                4 => {
                                    self.calls_.push(MockInterfaceCall::ReadOther(
                                        addr,
                                        self.other_read_value,
                                    ));
                                    into[0] = (self.other_read_value) as u8;
                                    into[1] = (self.other_read_value >> 8) as u8;
                                    into[2] = (self.other_read_value >> 16) as u8;
                                    into[3] = (self.other_read_value >> 24) as u8;
                                }
                                _ => {
                                    return Err(MockError("unsupported read length in mock"));
                                }
                            }
                            // We ignore all other writes because they aren't relevant
                            // to our coprocessor testing.
                        }
                    }
                    Ok(())
                }
                None => Err(MockError("continue_read without an active read")),
            }
        }

        fn end_read(&mut self) -> core::result::Result<(), Self::Error> {
            let result = match self.read_addr {
                Some(_) => Ok(()),
                None => Err(MockError("end_read without an active read")),
            };
            self.read_addr = None;
            result
        }

        fn host_cmd(
            &mut self,
            _cmd: u8,
            _a0: u8,
            _a1: u8,
        ) -> core::result::Result<(), Self::Error> {
            // Commands aren't relevant to our coprocessor tests.
            Ok(())
        }
    }

    impl PartialEq for MockInterfaceCall {
        fn eq(&self, other: &Self) -> bool {
            match self {
                MockInterfaceCall::ReadSpace(self_space) => {
                    if let MockInterfaceCall::ReadSpace(other_space) = other {
                        *self_space == *other_space
                    } else {
                        false
                    }
                }
                MockInterfaceCall::ReadWritePtr(self_v) => {
                    if let MockInterfaceCall::ReadWritePtr(other_v) = other {
                        *self_v == *other_v
                    } else {
                        false
                    }
                }
                MockInterfaceCall::ReadOther(self_addr, self_v) => {
                    if let MockInterfaceCall::ReadOther(other_addr, other_v) = other {
                        *self_addr == *other_addr && *self_v == *other_v
                    } else {
                        false
                    }
                }
                MockInterfaceCall::Write(self_data) => {
                    if let MockInterfaceCall::Write(other_data) = other {
                        self_data.eq(other_data)
                    } else {
                        false
                    }
                }
                MockInterfaceCall::StartStream => {
                    if let MockInterfaceCall::StartStream = other {
                        true
                    } else {
                        false
                    }
                }
                MockInterfaceCall::StopStream => {
                    if let MockInterfaceCall::StopStream = other {
                        true
                    } else {
                        false
                    }
                }
            }
        }
    }
}
