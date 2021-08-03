use crate::hal::{gpio::OutputPin, spi};
use nb;

pub struct WinbondW25q32jvFlash<SPI: spi::FullDuplex<u8>, P: OutputPin> {
    spi: SPI,
    chip_select: P,
}

pub enum Error {
    WrongManufacturerId,
    SpiError,
}

enum Command {
    ReadManufacturerDeviceId = 0x90,
}

const MANUFACTURER_AND_DEVICE_ID: &'static [u8] = &[0xEF, 0x40, 0x16];

struct DummyBytes(usize);

trait SpiHelpers {
    fn send_discarding_response(&mut self, byte: u8, dummy_bytes: DummyBytes) -> nb::Result<(), Error>;
    fn read_bytes(&mut self, bytes: &mut [u8]) -> nb::Result<(), Error>;
}

impl<SPI: spi::FullDuplex<u8>> SpiHelpers for SPI {
    fn send_discarding_response(&mut self, byte: u8, dummy_bytes: DummyBytes) -> nb::Result<(), Error> {
        self.transmit(Some(byte)).map_err(|_| Error::SpiError)?;
        self.receive().map_err(|_| Error::SpiError)?;

        for _ in 0..dummy_bytes.0 {
            self.transmit(Some(0x00)).map_err(|_| Error::SpiError)?;
            self.receive().map_err(|_| Error::SpiError)?;
        }

        Ok(())
    }

    fn read_bytes(&mut self, bytes: &mut [u8]) -> nb::Result<(), Error>{
        for byte in bytes {
            self.transmit(None).map_err(|_| Error::SpiError)?;
            *byte = self.receive().map_err(|_| Error::SpiError)?;
        }
        Ok(())
    }
}

impl<SPI: spi::FullDuplex<u8>, P: OutputPin> WinbondW25q32jvFlash<SPI, P> {
    pub fn new(spi: SPI, chip_select: P) -> nb::Result<Self, Error> {
        let mut flash = Self { spi, chip_select };
        flash.verify_id()?;
        Ok(flash)
    }

    fn verify_id(&mut self) -> nb::Result<(), Error> {
        self.chip_select.set_low();
        self.spi.send_discarding_response(Command::ReadManufacturerDeviceId as u8, DummyBytes(3))?;
        let mut response = [0u8; 3usize];
        self.spi.read_bytes(&mut response)?;
        self.chip_select.set_high();
        if response != MANUFACTURER_AND_DEVICE_ID {
            return Err(nb::Error::Other(Error::WrongManufacturerId));
        }
        Ok(())
    }
}
