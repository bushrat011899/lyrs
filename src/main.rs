//! Make some noise via cpal.
#![allow(clippy::precedence)]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use lyre::Lyre;
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

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
