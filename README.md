# Lyrs

A Lyre synth written in Rust.

## Setup

1. Ensure Rust is installed `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. Build and run via `cargo run --release`

### Troubleshooting

#### Rust wont install on my Raspberry Pi?

When installing Rust via `rustup`, ensure you set the "Default Host Triple" to `arm-unknown-linux-gnueabihf`.

#### `alsa-sys` wont compile?

On Linux based platforms, ensure you have `libasound2-dev` installed. For example:

```bash
sudo apt-get install libasound2-dev
```
