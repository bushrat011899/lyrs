//! Provides a driver for a Microchip MCP3004/3008 ADC via the `embedded-hal` ecosystem.

#![no_std]
#![forbid(unsafe_code)]

use embedded_hal::spi::SpiDevice;

#[cfg(feature = "mcp3004")]
pub mod mcp3004;

#[cfg(feature = "mcp3008")]
pub mod mcp3008;

/// Internal method for reading/writing to an MCP300 class chip. Channel must be valid for the intended chip.
pub(crate) fn read_with_mode<SPI: SpiDevice>(spi: &mut SPI, channel: u8, single_ended: bool) -> Result<u16, SPI::Error> {
    let mode = if single_ended {
        0b1000_0000
    } else {
        0b0000_0000
    };

    let channel = (channel & 0b111) << 4;

    let mut buffer = [0b0000_0001, mode | channel, 0b0000_0000];

    spi.transfer_in_place(&mut buffer)?;

    let result = u16::from_be_bytes([buffer[1], buffer[2]]);

    let result = result & 0b0000_0011_1111_1111;

    Ok(result)
}