#[cfg(feature = "raspberry_pi")]
mod imports {
    use std::cell::RefCell;
    use std::time::Duration;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use embedded_hal::spi::{Operation, SpiDevice};
    use embedded_hal_bus::spi::RefCellDevice;
    use lyre::Lyre;
    use mcp300::{Mcp300, mcp3008::{Channel, Mcp3008}};
    use rppal::gpio::Gpio;
    use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
}

#[cfg(feature = "raspberry_pi")]
use imports::*;

#[cfg(feature = "raspberry_pi")]
fn main() -> Result<(), anyhow::Error> {
    let gpio = Gpio::new()?;

    let mut spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 1_000_000, Mode::Mode0)?;

    let spi = RefCell::new(spi);

    let mcp = RefCellDevice::new_no_delay(&spi, gpio.get(24)?.into_output());

    let mut mcp = Mcp3008::new(mcp);

    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("Failed to find a default output device");

    let device = Box::leak::<'static>(Box::new(device));

    let supported_config = device.default_output_config().unwrap();

    let mut lyre = Lyre::new();

    let stream_builder = lyre.stream(&*device, &supported_config);

    // Spawn a thread to play the stream
    let _audio_thread = std::thread::spawn(move || {
        let stream = stream_builder();
        stream.play().unwrap();

        std::thread::sleep(std::time::Duration::MAX);
    });

    let mut plucking = false;

    loop {
        let readings = Channel::all()
            .enumerate()
            .filter_map(|(_, channel)| mcp.read(channel).ok());

        for (index, data) in readings {
            if data < 400 && !plucking {
                lyre.pluck(72.0 + index as f64);
                plucking = true;
            }

            if data > 900 && plucking {
                plucking = false;
            }
        }

        std::thread::sleep(Duration::from_millis(1));
    }
}

#[cfg(not(feature = "raspberry_pi"))]
fn main() {
    const _: () = panic!("Raspberry Pi feature must be enabled");
}