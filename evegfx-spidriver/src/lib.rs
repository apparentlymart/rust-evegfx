#![no_std]

use embedded_hal::serial::{Read, Write};
use evegfx::interface::{EVEAddress, EVECommand, EVEInterface};
use spidriver::SPIDriver;

pub struct EVESPIDriverInterface<TX, RX>
where
    TX: Write<u8>,
    RX: Read<u8>,
{
    sd: SPIDriver<TX, RX>,
}

impl<TX, RX> EVESPIDriverInterface<TX, RX>
where
    TX: Write<u8>,
    RX: Read<u8>,
{
    pub fn new(sd: SPIDriver<TX, RX>) -> Self {
        Self { sd: sd }
    }

    fn with_cs<F, R>(&mut self, func: F) -> Result<R, <Self as EVEInterface>::Error>
    where
        F: FnOnce(&mut Self) -> Result<R, <Self as EVEInterface>::Error>,
    {
        self.sd.select()?;
        let result = func(self);
        self.sd.unselect()?;
        result
    }
}

impl<TX, RX> EVEInterface for EVESPIDriverInterface<TX, RX>
where
    TX: Write<u8>,
    RX: Read<u8>,
{
    type Error = spidriver::Error<TX::Error, RX::Error>;

    fn write(&mut self, addr: EVEAddress, v: &[u8]) -> Result<(), Self::Error> {
        self.with_cs(|ei| {
            let mut addr_words: [u8; 3] = [0; 3];
            addr.build_write_header(&mut addr_words);
            ei.sd.write(&addr_words)?;
            ei.sd.write(v)
        })
    }

    fn read(&mut self, addr: EVEAddress, into: &mut [u8]) -> Result<(), Self::Error> {
        self.with_cs(|ei| {
            let mut addr_words: [u8; 4] = [0; 4];
            addr.build_read_header(&mut addr_words);
            ei.sd.write(&addr_words)?;
            ei.sd.transfer(into)?;
            return Ok(());
        })
    }

    fn cmd(&mut self, cmd: EVECommand, a0: u8, a1: u8) -> Result<(), Self::Error> {
        self.with_cs(|ei| {
            let mut cmd_words: [u8; 3] = [0; 3];
            cmd.build_message(a0, a1, &mut cmd_words);
            ei.sd.write(&cmd_words)
        })
    }
}
