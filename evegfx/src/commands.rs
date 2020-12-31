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

        // calls_ is the call log. Each call to a mock method appends one
        // entry to this vector, including any that fail.
        calls_: Vec<MockInterfaceCall>,
    }

    #[derive(Clone)]
    pub enum MockInterfaceCall {
        ReadSpace(u16),
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
                MockInterfaceCall::Write(v) => write!(f, "Write({:#010x?})", v),
                MockInterfaceCall::StartStream => write!(f, "StartStream"),
                MockInterfaceCall::StopStream => write!(f, "StopStream"),
            }
        }
    }

    impl MockInterface {
        const SPACE_ADDR: u32 = <Exhaustive as Model>::RegisterMem::BASE_ADDR + 0x574;
        const WRITE_ADDR: u32 = <Exhaustive as Model>::RegisterMem::BASE_ADDR + 0x578;

        pub fn new() -> Self {
            Self {
                write_addr: None,
                read_addr: None,
                current_space: 0xffc,
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
                    if addr == Self::SPACE_ADDR {
                        if into.len() != 2 {
                            return Err(MockError("must read REG_CMDB_SPACE with rd16"));
                        }
                        self.calls_
                            .push(MockInterfaceCall::ReadSpace(self.current_space));
                        into[0] = (self.current_space & 0xff) as u8;
                        into[1] = (self.current_space >> 8) as u8;
                    }
                    // We ignore all other writes because they aren't relevant
                    // to our coprocessor testing.
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
