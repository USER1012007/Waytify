use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub struct AudioPlayer {
    _stream: cpal::Stream,
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device().unwrap();
        let config = device.default_output_config()?;

        let stream = device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _| {
                for sample in data.iter_mut() {
                    *sample = 0.0;
                }
            },
            |err| eprintln!("audio error: {err}"),
            None,
        )?;

        stream.play()?;

        Ok(Self { _stream: stream })
    }
}
