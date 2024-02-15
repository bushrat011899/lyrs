//! Make some noise via cpal.
#![allow(clippy::precedence)]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, FromSample, SizedSample, Stream, SupportedStreamConfig};
use fundsp::hacker::*;
use funutd::Rnd;
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

#[cfg(feature = "raspberry_pi")]
mod raspberry_pi;

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

fn main() {
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

    let event_loop = EventLoop::new().unwrap();

    let window = WindowBuilder::new()
        .with_title("lyrs")
        .with_inner_size(winit::dpi::LogicalSize::new(256.0, 64.0))
        .build(&event_loop)
        .unwrap();

    event_loop
        .run(move |event, elwt| match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::RedrawRequested => {
                    window.pre_present_notify();
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            logical_key,
                            state: ElementState::Pressed,
                            repeat: false,
                            ..
                        },
                    ..
                } => {
                    let key = logical_key
                        .to_text()
                        .and_then(|text| text.chars().next())
                        .and_then(|char| char.to_digit(10))
                        .map(|number| 72. + number as f64);

                    if let Some(key) = key {
                        lyre.pluck(key);
                    }
                }
                _ => (),
            },
            Event::AboutToWait => {
                window.request_redraw();
            }

            _ => (),
        })
        .unwrap();
}
