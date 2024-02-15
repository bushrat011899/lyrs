use std::cell::RefCell;
use std::time::Duration;

use rppal::gpio::Gpio;
use rppal::spi::{Bus, Mode, Segment, SlaveSelect, Spi};
use embedded_hal::spi::{SpiBus, Operation, SpiDevice};

/// MCP3008 driver
pub struct Mcp3008<SPI> {
    spi: SPI,
}

impl<SPI: SpiDevice> Mcp3008<SPI> {
    /// Creates a new driver from an SPI peripheral.
    /// Please ensure the SPI bus is in SPI mode 0, aka (0, 0).
    pub fn new(spi: SPI) -> Self {
        Mcp3008 { spi }
    }

    /// Read a MCP3008 ADC channel and return the 10 bit value as a [`u16`] in single-ended mode.
    pub fn read(&mut self, ch: Channel) -> Result<u16, SPI::Error> {
        self.read_with_mode(ch, true)
    }

    /// Read a MCP3008 ADC channel and return the 10 bit value as a [`u16`].
    /// If `single_ended` is `true`, the conversion will be completed in single-ended mode.
    /// If `false`, the conversion will instead use differential mode.
    pub fn read_with_mode(&mut self, ch: Channel, single_ended: bool) -> Result<u16, SPI::Error> {
        // Message to send to initiate serial communication
        const START: &[u8] = &[0b0000_0001];

        // Message to send to select which channel to read and in what mode
        let message = {
            let mut message = 0;

            // Set Single Ended Mode if Enabled
            message <<= 1;
            if single_ended {
                message |= 1;
            }

            // Select Channel
            message <<= 3;
            message |= ch as u8 & 0b111;

            // Padding
            message <<= 4;

            message
        };

        // Buffer to store resulting 10-bit value as two bytes
        let mut buffer = [0; 2];

        let (high, low) = buffer.split_at_mut(1);

        self.spi.transaction(&mut [
            // Send start request
            Operation::Write(START),
            // Send request and read B9, B8
            Operation::Transfer(high, &[message]),
            // Read B7 - B0
            Operation::Read(low),
        ])?;

        // Discard null bit and other undefined bits
        buffer[0] &= 0b0000_0011;

        // Combine high and low bytes into a single u16
        let result = ((buffer[0] as u16) << 8) | (buffer[1] as u16);

        Ok(result)
    }
}

/// Channel list for MCP3008
#[allow(missing_docs)]
#[repr(u8)]
pub enum Channel {
    CH0 = 0,
    CH1 = 1,
    CH2 = 2,
    CH3 = 3,
    CH4 = 4,
    CH5 = 5,
    CH6 = 6,
    CH7 = 7,
}

fn main() -> Result<(), anyhow::Error> {
    let gpio = Gpio::new()?;

    let mut spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 1_000_000, Mode::Mode0)?;

    let spi = RefCell::new(spi);

    let mcp = RefCellDevice::new(&spi, gpio.get(24)?.into_output());

    let mut mcp = Mcp3008::new(spi);

    loop {
        let data = mcp.read(Channel::CH7)?;

        println!("Channel 7: {data}");

        std::thread::sleep(Duration::from_milis(500));
    }

    Ok(())
}
