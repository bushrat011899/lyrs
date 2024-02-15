use embedded_hal::spi::SpiBus;
use rppal::gpio::Gpio;
use rppal::spi::{Bus, Mode, Segment, SlaveSelect, Spi};

mod mcp3008;

use mcp3008::*;

fn test() -> Result<(), anyhow::Error> {
    let gpio = Gpio::new()?;

    let mut spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 1_000_000, Mode::Mode0)?;

    let spi = RefCell::new(spi);

    let mcp = RefCellDevice::new(&spi, gpio.get(23)?.into_output());

    let mut mcp = Mcp3008::new(spi);

    let data = mcp.read(Channel::CH0)?;

    Ok(())
}
