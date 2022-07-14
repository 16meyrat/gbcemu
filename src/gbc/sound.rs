use anyhow::{Context, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Sample,
};

pub struct Sound {
    device: Device,
}

impl Sound {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("no output device available")?;
        let mut supported_configs_range = device
            .supported_output_configs()
            .context("error while querying configs")?;
        let supported_config = supported_configs_range
            .next()
            .context("no supported config?!")?
            .with_max_sample_rate();
        use cpal::{SampleFormat};
        let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);
        let sample_format = supported_config.sample_format();
        let config = supported_config.into();
        let stream = match sample_format {
            SampleFormat::F32 => device.build_output_stream(&config, write_silence::<f32>, err_fn),
            SampleFormat::I16 => device.build_output_stream(&config, write_silence::<i16>, err_fn),
            SampleFormat::U16 => device.build_output_stream(&config, write_silence::<u16>, err_fn),
        }
        .context("Failed to build output audio stream")?;
        stream.play().context("Failed to play stream")?;
        Ok(Self { device })
    }
}

fn write_silence<T: Sample>(data: &mut [T], _: &cpal::OutputCallbackInfo) {
    for sample in data.iter_mut() {
        *sample = Sample::from(&0.0);
    }
}
