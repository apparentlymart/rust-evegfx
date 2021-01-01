//! Traits for binding the API from this crate to specific hardware platforms.

pub mod fake;

/// Implementations of `Interface` serve as adapters between the interface
/// this library expects and a specific physical implementation of that
/// interface, such as a SPI bus.
///
/// The main library contains no implementations of this trait, in order to
/// make the library portable across systems big and small. Other crates,
/// including some with the name prefix `evegfx`, take on additional
/// dependencies in order to bind this library to specific systems/hardware.
pub trait Interface: Sized {
    type Error;

    fn begin_write(&mut self, addr: u32) -> Result<(), Self::Error>;
    fn begin_read(&mut self, addr: u32) -> Result<(), Self::Error>;
    fn continue_write(&mut self, v: &[u8]) -> Result<(), Self::Error>;
    fn continue_read(&mut self, into: &mut [u8]) -> Result<(), Self::Error>;
    fn end_write(&mut self) -> Result<(), Self::Error>;
    fn end_read(&mut self) -> Result<(), Self::Error>;
    fn host_cmd(&mut self, cmd: u8, a0: u8, a1: u8) -> Result<(), Self::Error>;

    fn reset(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn write(&mut self, addr: u32, v: &[u8]) -> Result<(), Self::Error> {
        self.begin_write(addr)?;
        self.continue_write(v)?;
        self.end_write()
    }

    fn read(&mut self, addr: u32, into: &mut [u8]) -> Result<(), Self::Error> {
        self.begin_read(addr)?;
        self.continue_read(into)?;
        self.end_read()
    }

    /// Write the three bytes needed to form a "write memory" header
    /// for the address into the given bytes. This is a helper for
    /// physical implementations that need to construct a message
    /// buffer to transmit to the real chip, e.g. via SPI.
    fn build_write_header(&self, addr: u32, into: &mut [u8; 3]) {
        into[0] = (((addr >> 16) & 0b00111111) | 0b10000000) as u8;
        into[1] = (addr >> 8) as u8;
        into[2] = (addr >> 0) as u8;
    }

    /// Write the four bytes needed to form a "read memory" header
    /// for the address into the given bytes. This is a helper for
    /// physical implementations that need to construct a message
    /// buffer to transmit to the real chip, e.g. via SPI.
    fn build_read_header(&self, addr: u32, into: &mut [u8; 4]) {
        into[0] = ((addr >> 16) & 0b00111111) as u8;
        into[1] = (addr >> 8) as u8;
        into[2] = (addr >> 0) as u8;
        into[3] = 0; // "dummy byte", per the datasheet
    }

    /// Write the three bytes needed to form a command message
    /// for the command and two arguments given. This is a helper
    /// for physical implementations that need to construct a
    /// message buffer to transmit to the real chip, e.g. via SPI.
    fn build_host_cmd_msg(&self, mut cmd: u8, a0: u8, a1: u8, into: &mut [u8; 3]) {
        // Make sure the command conforms to the expected bit pattern so that
        // it won't be misunderstood as a read or write.
        // Command zero, ACTIVE, is an exception that's intentionally encoded
        // to look the same as a read header for address zero.
        if cmd != 0 {
            cmd = (cmd & 0b00111111) | 0b01000000;
        }
        into[0] = cmd;
        into[1] = a0;
        into[2] = a1;
    }
}

/// Read the raw chip ID data from the given interface. This is a helper
/// for callers of the lowest-level interface API. Higher layers may
/// provide a more abstract form of this helper which interpret the raw
/// ID data in some way.
///
/// The chip ID data is in the general RAM space, so you should read it
/// only during early startup, before an application has potentially written
/// other data over the top of it.
pub fn read_chip_id<I: Interface>(ei: &mut I) -> Result<[u8; 4], I::Error> {
    let mut into: [u8; 4] = [0; 4];
    ei.read(0xC0000, &mut into)?;
    Ok(into)
}

// We use std in test mode only, so we can do dynamic allocation in the mock
// code.
#[cfg(test)]
pub mod testing {
    extern crate std;

    use super::Interface;
    use std::collections::HashMap;
    use std::vec::Vec;

    /// A test double for `trait Interface`, available only in test mode.
    pub struct MockInterface {
        _write_addr: Option<u32>,
        _read_addr: Option<u32>,

        // _mem is a sparse representation of the memory space which
        // remembers what was written into it and returns 0xff if asked
        // for an address that wasn't previously written.
        _mem: HashMap<u32, u8>,

        // if _fail is Some then the mock methods will call it and use the
        // result to decide whether to return an error.
        _fail: Option<fn(&MockInterfaceCall) -> bool>,

        // _calls is the call log. Each call to a mock method appends one
        // entry to this vector, including any that fail.
        _calls: Vec<MockInterfaceCall>,
    }

    #[derive(Clone, Debug)]
    pub enum MockInterfaceCall {
        BeginWrite(u32),
        ContinueWrite(Vec<u8>),
        EndWrite(u32),
        BeginRead(u32),
        ContinueRead(usize),
        EndRead(u32),
        Cmd(u8, u8, u8),
    }

    impl MockInterface {
        const DEFAULT_MEM_VALUE: u8 = 0xff;

        pub fn new() -> Self {
            Self {
                _write_addr: None,
                _read_addr: None,
                _mem: HashMap::new(),
                _fail: None,
                _calls: Vec::new(),
            }
        }

        /// Consumes the mock and returns all of the calls it logged
        /// during its life.
        pub fn calls(self) -> Vec<MockInterfaceCall> {
            self._calls
        }

        // Copies some data into the fake memory without considering it
        // to be a logged operation. This is intended for setting up
        // memory ready for subsequent calls to `read`.
        pub fn setup_mem(&mut self, addr: u32, buf: &[u8]) {
            for (i, v) in buf.iter().enumerate() {
                let e_addr = addr + i as u32;
                let v = *v; // Take a copy of the value from the input
                if v == Self::DEFAULT_MEM_VALUE {
                    self._mem.remove(&e_addr);
                } else {
                    self._mem.insert(e_addr, v);
                }
            }
        }

        fn call_should_fail(&self, call: &MockInterfaceCall) -> bool {
            if let Some(fail) = self._fail {
                if fail(&call) {
                    return true;
                }
            }
            return false;
        }
    }

    #[derive(Debug)]
    pub struct MockError();

    impl Interface for MockInterface {
        type Error = MockError;

        fn begin_write(&mut self, addr: u32) -> core::result::Result<(), Self::Error> {
            let call = MockInterfaceCall::BeginWrite(addr);
            if self.call_should_fail(&call) {
                self._calls.push(call);
                return Err(MockError());
            }
            self._calls.push(call);
            self._write_addr = Some(addr);
            Ok(())
        }

        fn continue_write(&mut self, buf: &[u8]) -> core::result::Result<(), Self::Error> {
            let log_buf = buf.to_vec();
            let call = MockInterfaceCall::ContinueWrite(log_buf);
            if self.call_should_fail(&call) {
                self._calls.push(call);
                return Err(MockError());
            }
            self._calls.push(call);

            let addr = self._write_addr.unwrap();
            self.setup_mem(addr, buf);
            Ok(())
        }

        fn end_write(&mut self) -> core::result::Result<(), Self::Error> {
            let addr = self._write_addr.unwrap();
            let call = MockInterfaceCall::EndWrite(addr);
            if self.call_should_fail(&call) {
                self._calls.push(call);
                return Err(MockError());
            }
            self._calls.push(call);
            self._write_addr = None;
            Ok(())
        }

        fn begin_read(&mut self, addr: u32) -> core::result::Result<(), Self::Error> {
            let call = MockInterfaceCall::BeginRead(addr);
            if self.call_should_fail(&call) {
                self._calls.push(call);
                return Err(MockError());
            }
            self._calls.push(call);
            self._read_addr = Some(addr);
            Ok(())
        }

        fn continue_read(&mut self, into: &mut [u8]) -> core::result::Result<(), Self::Error> {
            let call = MockInterfaceCall::ContinueRead(into.len());
            if self.call_should_fail(&call) {
                self._calls.push(call);
                return Err(MockError());
            }
            self._calls.push(call);

            let addr = self._read_addr.unwrap();
            for i in 0..into.len() {
                let e_addr = addr + (i as u32);
                let v = self
                    ._mem
                    .get(&e_addr)
                    .unwrap_or(&Self::DEFAULT_MEM_VALUE)
                    .clone();
                into[i] = v;
            }
            Ok(())
        }

        fn end_read(&mut self) -> core::result::Result<(), Self::Error> {
            let addr = self._read_addr.unwrap();
            let call = MockInterfaceCall::EndRead(addr);
            if self.call_should_fail(&call) {
                self._calls.push(call);
                return Err(MockError());
            }
            self._calls.push(call);
            self._read_addr = None;
            Ok(())
        }

        fn host_cmd(&mut self, cmd: u8, a0: u8, a1: u8) -> core::result::Result<(), Self::Error> {
            let call = MockInterfaceCall::Cmd(cmd, a0, a1);
            if let Some(fail) = self._fail {
                if fail(&call) {
                    self._calls.push(call);
                    return Err(MockError());
                }
            }
            self._calls.push(call);
            Ok(())
        }
    }

    impl PartialEq for MockInterfaceCall {
        fn eq(&self, other: &Self) -> bool {
            match self {
                MockInterfaceCall::BeginWrite(self_addr) => {
                    if let MockInterfaceCall::BeginWrite(other_addr) = other {
                        *self_addr == *other_addr
                    } else {
                        false
                    }
                }
                MockInterfaceCall::ContinueWrite(self_data) => {
                    if let MockInterfaceCall::ContinueWrite(other_data) = other {
                        self_data.eq(other_data)
                    } else {
                        false
                    }
                }
                MockInterfaceCall::EndWrite(self_addr) => {
                    if let MockInterfaceCall::EndWrite(other_addr) = other {
                        *self_addr == *other_addr
                    } else {
                        false
                    }
                }
                MockInterfaceCall::BeginRead(self_addr) => {
                    if let MockInterfaceCall::BeginRead(other_addr) = other {
                        *self_addr == *other_addr
                    } else {
                        false
                    }
                }
                MockInterfaceCall::ContinueRead(self_len) => {
                    if let MockInterfaceCall::ContinueRead(other_len) = other {
                        *self_len == *other_len
                    } else {
                        false
                    }
                }
                MockInterfaceCall::EndRead(self_addr) => {
                    if let MockInterfaceCall::EndRead(other_addr) = other {
                        *self_addr == *other_addr
                    } else {
                        false
                    }
                }
                MockInterfaceCall::Cmd(self_cmd, self_a0, self_a1) => {
                    if let MockInterfaceCall::Cmd(other_cmd, other_a0, other_a1) = other {
                        *self_cmd == *other_cmd && *self_a0 == *other_a0 && *self_a1 == *other_a1
                    } else {
                        false
                    }
                }
            }
        }
    }
}
