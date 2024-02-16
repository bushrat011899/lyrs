use embedded_hal::spi::SpiDevice;

use crate::read_with_mode;

/// MCP3004 driver
pub struct Mcp3004<SPI> {
    spi: SPI,
}

impl<SPI: SpiDevice> Mcp3004<SPI> {
    /// Creates a new driver from an SPI peripheral.
    /// Please ensure the SPI bus is in SPI mode 0, aka (0, 0).
    pub fn new(spi: SPI) -> Self {
        spi.into()
    }

    /// Read a channel and return the 10 bit value as a [`u16`].
    /// If `single_ended` is `true`, the conversion will be completed in single-ended mode.
    /// If `false`, the conversion will instead use differential mode.
    pub fn read_with_mode(
        &mut self,
        ch: Channel,
        single_ended: bool,
    ) -> Result<u16, SPI::Error> {
        read_with_mode(&mut self.spi, ch as u8, single_ended)
    }

    /// Read a channel and return the 10 bit value as a [`u16`] in single-ended mode.
    pub fn read(&mut self, ch: Channel) -> Result<u16, SPI::Error> {
        self.read_with_mode(ch, true)
    }

    /// Read a channel and return the 10 bit value as a [`u16`] in single-ended mode.
    pub fn read_differential(&mut self, ch: Channel) -> Result<u16, SPI::Error> {
        self.read_with_mode(ch, false)
    }
}

impl<SPI: SpiDevice> From<SPI> for Mcp3004<SPI> {
    fn from(spi: SPI) -> Self {
        Self { spi }
    }
}

/// Channel list for MCP3004
#[allow(missing_docs)]
#[repr(u8)]
pub enum Channel {
    CH0 = 0,
    CH1 = 1,
    CH2 = 2,
    CH3 = 3,
}

impl Channel {
    /// Iterate over all channels.
    pub fn all() -> impl Iterator<Item = Self> {
        [Self::CH0, Self::CH1, Self::CH2, Self::CH3].into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_hal::spi::{Error, ErrorKind, ErrorType, Operation};

    #[test]
    fn mock_spi() {
        #[derive(Debug, PartialEq)]
        struct MockError;

        impl Error for MockError {
            fn kind(&self) -> ErrorKind {
                ErrorKind::Other
            }
        }

        struct MockSpi;

        impl ErrorType for MockSpi {
            type Error = MockError;
        }

        impl SpiDevice for MockSpi {
            fn transaction(
                &mut self,
                operations: &mut [Operation<'_, u8>],
            ) -> Result<(), Self::Error> {
                assert_eq!(operations.len(), 1);

                match &mut operations[0] {
                    Operation::TransferInPlace(words) => {
                        assert_eq!(words[0], 0b0000_0001, "Missing start flag");

                        words[1] &= 0b0111_0000;

                        words[2] = match words[1] >> 4 {
                            0 => 100,
                            1 => 101,
                            2 => 102,
                            3 => 103,
                            _ => unreachable!(),
                        };
                    }
                    _ => panic!("Not an expected operation"),
                }

                Ok(())
            }
        }

        let mut mcp = Mcp3004::new(MockSpi);

        assert_eq!(mcp.read(Channel::CH0), Ok(100));
        assert_eq!(mcp.read(Channel::CH1), Ok(101));
        assert_eq!(mcp.read(Channel::CH2), Ok(102));
        assert_eq!(mcp.read(Channel::CH3), Ok(103));
    }
}
