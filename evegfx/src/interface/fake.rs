//! Fake `Interface` implementation for testing and examples.

use core::convert::Infallible;

/// An implementation of [`Interface`](super::Interface) which just reads and
/// writes a buffer in local RAM.
///
/// This type alone doesn't implement any of the typical functionality that
/// would be expected from an EVE chip, but it could in principle be used in
/// conjunction with other code (probably running in a separate thread) to
/// simulate parts of the EVE functionality for testing purposes.
///
/// This is mainly here just so there's a simple backend to write tests and
/// examples against.
pub struct Interface<'a, WriteCallback, WriteError>
where
    WriteError: core::fmt::Debug,
    WriteCallback: Fn(u32, &[u8]) -> Result<(), WriteError>,
{
    storage: &'a mut [u8],
    write_callback: WriteCallback,

    write_addr: Option<u32>,
    read_addr: Option<u32>,
}

impl<'a> Interface<'a, NoCallbackType, Infallible> {
    pub fn new(storage: &'a mut [u8]) -> Self {
        Self {
            storage: storage,
            write_callback: no_callback,
            write_addr: None,
            read_addr: None,
        }
    }
}

impl<'a, WriteCallback, WriteError> Interface<'a, WriteCallback, WriteError>
where
    WriteError: core::fmt::Debug,
    WriteCallback: Fn(u32, &[u8]) -> Result<(), WriteError>,
{
    pub fn new_with_write_callback(storage: &'a mut [u8], callback: WriteCallback) -> Self {
        Self {
            storage: storage,
            write_callback: callback,
            write_addr: None,
            read_addr: None,
        }
    }
}

impl<'a, WriteCallback, WriteError> super::Interface for Interface<'a, WriteCallback, WriteError>
where
    WriteError: core::fmt::Debug,
    WriteCallback: Fn(u32, &[u8]) -> Result<(), WriteError>,
{
    type Error = Error<WriteError>;

    fn begin_write(&mut self, addr: u32) -> core::result::Result<(), Self::Error> {
        if let Some(_) = self.write_addr {
            return Err(Error::IncorrectSequence);
        }
        if let Some(_) = self.read_addr {
            return Err(Error::IncorrectSequence);
        }
        self.write_addr = Some(addr);
        Ok(())
    }

    fn continue_write(&mut self, data: &[u8]) -> core::result::Result<(), Self::Error> {
        if let Some(addr) = self.write_addr {
            for (i, v) in data.iter().enumerate() {
                let ptr = addr as usize + i;
                if ptr > self.storage.len() {
                    return Err(Error::OutOfBounds);
                }
                self.storage[ptr] = *v;
            }
            self.write_addr = Some(addr + self.storage.len() as u32);
            match (self.write_callback)(addr, data) {
                Err(err) => Err(Error::WriteCallback(err)),
                Ok(_) => Ok(()),
            }
        } else {
            Err(Error::IncorrectSequence)
        }
    }

    fn end_write(&mut self) -> core::result::Result<(), Self::Error> {
        if let Some(_) = self.write_addr {
            self.write_addr = None;
            Ok(())
        } else {
            Err(Error::IncorrectSequence)
        }
    }

    fn begin_read(&mut self, addr: u32) -> core::result::Result<(), Self::Error> {
        if let Some(_) = self.write_addr {
            return Err(Error::IncorrectSequence);
        }
        if let Some(_) = self.read_addr {
            return Err(Error::IncorrectSequence);
        }
        self.read_addr = Some(addr);
        Ok(())
    }

    fn continue_read(&mut self, into: &mut [u8]) -> core::result::Result<(), Self::Error> {
        if let Some(addr) = self.read_addr {
            for i in 0..into.len() {
                let ptr = addr as usize + i;
                if ptr > self.storage.len() {
                    return Err(Error::OutOfBounds);
                }
                into[ptr] = self.storage[i];
            }
            self.read_addr = Some(addr + self.storage.len() as u32);
            Ok(())
        } else {
            Err(Error::IncorrectSequence)
        }
    }

    fn end_read(&mut self) -> core::result::Result<(), Self::Error> {
        if let Some(_) = self.read_addr {
            self.read_addr = None;
            Ok(())
        } else {
            Err(Error::IncorrectSequence)
        }
    }

    fn host_cmd(&mut self, _cmd: u8, _a0: u8, _a1: u8) -> core::result::Result<(), Self::Error> {
        // For now the fake interface doesn't do anything with commands.
        Ok(())
    }
}

pub enum Error<WriteCallbackError> {
    OutOfBounds,
    WriteCallback(WriteCallbackError),
    IncorrectSequence,
}

type NoCallbackType = fn(u32, &[u8]) -> Result<(), Infallible>;

fn no_callback(_addr: u32, _data: &[u8]) -> Result<(), Infallible> {
    Ok(())
}
