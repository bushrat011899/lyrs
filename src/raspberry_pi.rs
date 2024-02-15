use std::cell::RefCell;
use std::time::Duration;

use rppal::gpio::Gpio;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use embedded_hal::spi::{Operation, SpiDevice};
use embedded_hal_bus::spi::RefCellDevice;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, FromSample, SizedSample, Stream, SupportedStreamConfig};
use fundsp::hacker::*;
use funutd::Rnd;
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

pub struct Lyre {
    sequencer: Sequencer64,
    rnd: Rnd,
}

impl Default for Lyre {
    fn default() -> Self {
        Self {
            sequencer: Sequencer64::new(false, 1),
            rnd: Rnd::from_time(),
        }
    }
}

impl Lyre {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stream<'a, 'b>(
        &'a mut self,
        device: &'static Device,
        config: &'a SupportedStreamConfig,
    ) -> impl FnOnce() -> Stream + 'b {
        let sequencer_backend = self.sequencer.backend();
        let config = config.clone();

        move || match config.sample_format() {
            cpal::SampleFormat::F32 => {
                Self::run::<f32>(device, &config.into(), sequencer_backend).unwrap()
            }
            cpal::SampleFormat::I16 => {
                Self::run::<i16>(device, &config.into(), sequencer_backend).unwrap()
            }
            cpal::SampleFormat::U16 => {
                Self::run::<u16>(device, &config.into(), sequencer_backend).unwrap()
            }
            _ => panic!("Unsupported format"),
        }
    }

    pub fn pluck(&mut self, midi: f64) {
        let waveform = Net64::wrap(Box::new(
            (brown() * lfo(|t| exp(-10. * t))) >> pluck(midi_hz(midi), 0.2, 0.2) * 0.5,
        ));

        let mut note = Box::new(waveform);

        note.ping(false, AttoHash::new(self.rnd.u64()));

        self.sequencer
            .push_relative(0.0, 5.0, Fade::Smooth, 0.02, 0.2, note);
    }

    fn run<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        sequencer_backend: SequencerBackend64,
    ) -> Result<Stream, anyhow::Error>
    where
        T: SizedSample + FromSample<f64>,
    {
        let sample_rate = config.sample_rate.0 as f64;
        let channels = config.channels as usize;

        let mut net = Net64::wrap(Box::new(sequencer_backend));

        net = net >> resonator_hz(925., 500.) >> pan(0.0);

        net.set_sample_rate(sample_rate);

        net.allocate();

        let mut next_value = move || net.get_stereo();

        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    let sample = next_value();
                    let left = T::from_sample(sample.0);
                    let right: T = T::from_sample(sample.1);

                    for (channel, sample) in frame.iter_mut().enumerate() {
                        if channel & 1 == 0 {
                            *sample = left;
                        } else {
                            *sample = right;
                        }
                    }
                }
            },
            |err| eprintln!("an error occurred on stream: {}", err),
            None,
        )?;

        Ok(stream)
    }
}

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

        let mut buffer = [0; 3];

        buffer[0] = 1;
        buffer[1] = 0b1111_0000;

        self.spi.transaction(&mut [
            Operation::TransferInPlace(&mut buffer)
        ])?;

        // Discard null bit and other undefined bits
        buffer[1] &= 0b0000_0011;

        // Combine high and low bytes into a single u16
        let result = ((buffer[1] as u16) << 8) | (buffer[2] as u16);

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
        let data = mcp.read(Channel::CH7).unwrap();

        if data < 400 && !plucking {
            println!("Plucked!");
            lyre.pluck(72.0);
            plucking = true;
        }

        if data > 900 && plucking {
            plucking = false;
        }

        std::thread::sleep(Duration::from_millis(1));
    }
}
