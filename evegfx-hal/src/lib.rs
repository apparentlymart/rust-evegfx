#![no_std]

use embedded_hal::blocking::spi::{Transfer, Write};
use embedded_hal::digital::v2::OutputPin;
use evegfx::interface::{EVECommand, Interface};

/// `EVEHALSPIInterface` is an implementation of `evegfx.Interface` that
/// commincates over SPI using the `embedded-hal` SPI and GPIO (for
/// "chip select") traits.
pub struct EVEHALSPIInterface<SPI, CS>
where
    SPI: Transfer<u8>,
    CS: OutputPin,
{
    spi: SPI,
    cs: CS,
}

impl<SPI, CS> EVEHALSPIInterface<SPI, CS>
where
    SPI: Transfer<u8> + Write<u8>,
    CS: OutputPin,
{
    /// Create a new EVE interface in terms of the given SPI bus and CS
    /// signal implementations.
    ///
    /// The given CS implementation must be a digital output pin which will be
    /// set to low to assert chip select, or high to unassert it, reflecting
    /// the physical characteristics of the CS pin on EVE IC packages.
    pub fn new(spi: SPI, cs: CS) -> Self {
        Self { spi: spi, cs: cs }
    }

    fn with_cs<F, R>(&mut self, func: F) -> Result<R, <Self as Interface>::Error>
    where
        F: FnOnce(&mut Self) -> Result<R, <Self as Interface>::Error>,
    {
        self.spi_select()?;
        let result = func(self);
        self.spi_unselect()?;
        result
    }

    fn spi_select(&mut self) -> Result<(), <Self as Interface>::Error> {
        <Self as Interface>::Error::cs_result(self.cs.set_low())
    }

    fn spi_unselect(&mut self) -> Result<(), <Self as Interface>::Error> {
        <Self as Interface>::Error::cs_result(self.cs.set_high())
    }

    fn spi_write(&mut self, words: &[u8]) -> Result<(), <Self as Interface>::Error> {
        let r = self.spi.write(words);
        self.spi_write_result(r)
    }

    fn spi_transfer<'w>(
        &mut self,
        words: &'w mut [u8],
    ) -> Result<&'w [u8], <Self as Interface>::Error> {
        let r = self.spi.transfer(words);
        self.spi_transfer_result(r)
    }

    fn spi_write_result<T>(
        &self,
        r: Result<T, <SPI as Write<u8>>::Error>,
    ) -> Result<T, <Self as Interface>::Error> {
        <Self as Interface>::Error::spi_write_result(r)
    }

    fn spi_transfer_result<T>(
        &self,
        r: Result<T, <SPI as Transfer<u8>>::Error>,
    ) -> Result<T, <Self as Interface>::Error> {
        <Self as Interface>::Error::spi_transfer_result(r)
    }
}

impl<SPI, CS> Interface for EVEHALSPIInterface<SPI, CS>
where
    SPI: Transfer<u8> + Write<u8>,
    CS: OutputPin,
{
    type Error = EVEHALSPIError<<SPI as Write<u8>>::Error, <SPI as Transfer<u8>>::Error, CS::Error>;

    fn begin_write(&mut self, addr: u32) -> Result<(), Self::Error> {
        self.spi_select()?;
        let mut addr_words: [u8; 3] = [0; 3];
        self.build_write_header(addr, &mut addr_words);
        self.spi_write(&addr_words)
    }

    fn continue_write(&mut self, v: &[u8]) -> Result<(), Self::Error> {
        self.spi_write(v)
    }

    fn end_write(&mut self) -> Result<(), Self::Error> {
        self.spi_unselect()
    }

    fn begin_read(&mut self, addr: u32) -> Result<(), Self::Error> {
        self.spi_select()?;
        let mut addr_words: [u8; 4] = [0; 4];
        self.build_read_header(addr, &mut addr_words);
        self.spi_write(&addr_words)
    }

    fn continue_read(&mut self, into: &mut [u8]) -> Result<(), Self::Error> {
        self.spi_transfer(into)?;
        Ok(())
    }

    fn end_read(&mut self) -> Result<(), Self::Error> {
        self.spi_unselect()
    }

    fn cmd(&mut self, cmd: EVECommand, a0: u8, a1: u8) -> Result<(), Self::Error> {
        self.with_cs(|ei| {
            let mut cmd_words: [u8; 3] = [0; 3];
            cmd.build_message(a0, a1, &mut cmd_words);
            ei.spi_write(&cmd_words)
        })
    }
}

pub enum EVEHALSPIError<SPIWriteError, SPITransferError, CSError> {
    SPIWrite(SPIWriteError),
    SPITransfer(SPITransferError),
    CS(CSError),
}

impl<SPIWriteError, SPITransferError, CSError>
    EVEHALSPIError<SPIWriteError, SPITransferError, CSError>
{
    fn spi_write_result<T>(r: Result<T, SPIWriteError>) -> Result<T, Self> {
        match r {
            Ok(v) => Ok(v),
            Err(e) => Err(Self::SPIWrite(e)),
        }
    }

    fn spi_transfer_result<T>(r: Result<T, SPITransferError>) -> Result<T, Self> {
        match r {
            Ok(v) => Ok(v),
            Err(e) => Err(Self::SPITransfer(e)),
        }
    }

    fn cs_result<T>(r: Result<T, CSError>) -> Result<T, Self> {
        match r {
            Ok(v) => Ok(v),
            Err(e) => Err(Self::CS(e)),
        }
    }
}
