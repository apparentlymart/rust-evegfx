#![no_std]

use embedded_hal::blocking::spi::{Transfer, Write};
use embedded_hal::digital::v2::OutputPin;
use evegfx::interface::{EVEAddress, EVECommand, EVEInterface};

/// `EVEHALSPIInterface` is an implementation of `evegfx.EVEInterface` that
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

    fn with_cs<F, R>(&mut self, func: F) -> Result<R, <Self as EVEInterface>::Error>
    where
        F: FnOnce(&mut Self) -> Result<R, <Self as EVEInterface>::Error>,
    {
        <Self as EVEInterface>::Error::cs_result(self.cs.set_low())?;
        let result = func(self);
        <Self as EVEInterface>::Error::cs_result(self.cs.set_high())?;
        result
    }

    fn spi_write(&mut self, words: &[u8]) -> Result<(), <Self as EVEInterface>::Error> {
        let r = self.spi.write(words);
        self.spi_write_result(r)
    }

    fn spi_transfer<'w>(
        &mut self,
        words: &'w mut [u8],
    ) -> Result<&'w [u8], <Self as EVEInterface>::Error> {
        let r = self.spi.transfer(words);
        self.spi_transfer_result(r)
    }

    fn spi_write_result<T>(
        &self,
        r: Result<T, <SPI as Write<u8>>::Error>,
    ) -> Result<T, <Self as EVEInterface>::Error> {
        <Self as EVEInterface>::Error::spi_write_result(r)
    }

    fn spi_transfer_result<T>(
        &self,
        r: Result<T, <SPI as Transfer<u8>>::Error>,
    ) -> Result<T, <Self as EVEInterface>::Error> {
        <Self as EVEInterface>::Error::spi_transfer_result(r)
    }
}

impl<SPI, CS> EVEInterface for EVEHALSPIInterface<SPI, CS>
where
    SPI: Transfer<u8> + Write<u8>,
    CS: OutputPin,
{
    type Error = EVEHALSPIError<<SPI as Write<u8>>::Error, <SPI as Transfer<u8>>::Error, CS::Error>;

    fn write(&mut self, addr: EVEAddress, v: &[u8]) -> Result<(), Self::Error> {
        self.with_cs(|ei| {
            let mut addr_words: [u8; 3] = [0; 3];
            addr.build_write_header(&mut addr_words);
            ei.spi_write(&addr_words)?;
            ei.spi_write(v)
        })
    }

    fn read(&mut self, addr: EVEAddress, into: &mut [u8]) -> Result<(), Self::Error> {
        self.with_cs(|ei| {
            let mut addr_words: [u8; 4] = [0; 4];
            addr.build_read_header(&mut addr_words);
            ei.spi_write(&addr_words)?;
            ei.spi_transfer(into)?;
            return Ok(());
        })
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
