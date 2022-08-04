use std::sync::mpsc::{channel, Receiver, Sender};

use anyhow::{Context, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, Sample, SampleRate, Stream, StreamConfig, SupportedBufferSize,
};

use super::bus::Busable;

pub struct Sound {
    _stream: Stream,
    state: SynthRegState,
    tx: Sender<SynthRegState>,
}

impl Sound {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("no output device available")?;

        /*
        let mut supported_configs_range = device
            .supported_output_configs()
            .context("error while querying configs")?;
        for a in supported_configs_range {
            println!("Supported audio: {:?}", a);
        }
        */

        let mut supported_configs_range = device
            .supported_output_configs()
            .context("error while querying configs")?;

        let supported_config = supported_configs_range
            .find(|c| c.channels() == 2 && matches!(c.sample_format(), SampleFormat::F32))
            .context("no suitable config")?
            .with_sample_rate(SampleRate(51200));

        #[cfg(feature="audio-log")]
        println!("Supported audio config: {supported_config:?}");

        use cpal::SampleFormat;
        let sample_format = supported_config.sample_format();
        let min_bufsize = match supported_config.buffer_size() {
            &SupportedBufferSize::Range { min, max: _ } => min,
            _ => 0,
        };

        let mut config: StreamConfig = supported_config.into();
        config.buffer_size = BufferSize::Fixed((1024).max(min_bufsize));

        #[cfg(feature="audio-log")]
        println!("Audio config: {config:?}");

        let (synth, tx) = new_synth(&config);
        let state = Default::default();

        let stream = match sample_format {
            SampleFormat::F32 => start_audio_stream::<f32>(&device, &config, synth),
            SampleFormat::I16 => start_audio_stream::<i16>(&device, &config, synth),
            SampleFormat::U16 => start_audio_stream::<u16>(&device, &config, synth),
        }
        .context("Failed to build output audio stream")?;
        stream.play().context("Failed to play stream")?;
        Ok(Self {
            _stream: stream,
            tx,
            state,
        })
    }
}

impl Busable for Sound {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0xff10 => 0x80 | self.state.sweep_time_1 | (self.state.negate_1 as u8) << 3,
            _ => 0, // TODO: panic
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        #[cfg(feature="audio-log")]
        println!("Audio write ({value:#x})to {addr:#x}");
        match addr {
            0xff10 => {
                self.state.sweep_shift_1 = value & 0x07;
                self.state.negate_1 = value & 0x08 != 0;
                self.state.sweep_time_1 = (value & 0xf0) >> 4;
            }
            0xff11 => {
                self.state.wave_pattern_1 = value >> 6;
                self.state.sound_length_1 = 64 - (value & 0x3f);
            }
            0xff12 => {
                self.state.envelope_vol_1 = value >> 4;
                self.state.envelope_increase_1 = value & 0x08 != 0;
                self.state.envelope_sweep_1 = value & 0x07;
            }
            0xff13 => {
                self.state.frequency_1 = self.state.frequency_1 & 0xff00 | value as u16;
            }
            0xff14 => {
                self.state.frequency_1 =
                    self.state.frequency_1 & 0x00ff | ((value & 0x7) as u16) << 8;
                self.state.trigger_1 = value & 0x80 != 0;
                self.state.length_en_1 = value & 0x40 != 0;
            }
            0xff15 => {
                eprintln!("Write to unused sound reguister ff15");
            }
            0xff16 => {
                self.state.wave_pattern_2 = value >> 6;
                self.state.sound_length_2 = 64 - (value & 0x3f);
            }
            0xff17 => {
                self.state.envelope_vol_2 = value >> 4;
                self.state.envelope_increase_2 = value & 0x08 != 0;
                self.state.envelope_sweep_2 = value & 0x07;
            }
            0xff18 => {
                self.state.frequency_2 = self.state.frequency_1 & 0xff00 | value as u16;
            }
            0xff19 => {
                self.state.frequency_2 =
                    self.state.frequency_2 & 0x00ff | ((value & 0x7) as u16) << 8;
                self.state.trigger_2 = value & 0x80 != 0;
                self.state.length_en_2 = value & 0x40 != 0;
            }
            0xff24 => {
                self.state.left_vol = (value & 0x70) >> 4;
                self.state.right_vol = value & 0x07;
            }
            0xff25 => {
                self.state.channel_pan = value;
            }
            0xff26 => {
                self.state.sound_enable = value & 0x80 != 0;
            }
            _ => {} // TODO: panic
        }
        self.tx
            .send(self.state.clone())
            .expect("Failed to send SyntCmd to audio thread");
        self.state.trigger_1 = false;
        self.state.trigger_2 = false;
    }
}

#[derive(Clone, Default)]
struct SynthRegState {
    sound_enable: bool,

    sweep_time_1: u8,
    negate_1: bool,
    sweep_shift_1: u8,

    wave_pattern_1: u8,
    sound_length_1: u8,

    envelope_vol_1: u8,
    envelope_increase_1: bool,
    envelope_sweep_1: u8,

    frequency_1: u16,

    trigger_1: bool,
    length_en_1: bool,

    wave_pattern_2: u8,
    sound_length_2: u8,

    envelope_vol_2: u8,
    envelope_increase_2: bool,
    envelope_sweep_2: u8,

    frequency_2: u16,

    trigger_2: bool,
    length_en_2: bool,

    channel_pan: u8,
    left_vol: u8,
    right_vol: u8,
}

const SQUARE_PATTERN: [[f32;8];4] = [
    [-1., -1., -1., -1., -1., -1., -1., 1.],
    [-1., -1., -1., -1., -1., -1., 1., 1.],
    [-1., -1., -1., -1., 1., 1., 1., 1.],
    [1., 1., 1., 1., 1., 1., -1., -1.]
];

fn start_audio_stream<T: Sample>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut synth: Synth,
) -> Result<Stream> {
    let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| audio_thread(data, &mut synth),
            err_fn,
        )
        .context("Failed start audio thread")?;
    Ok(stream)
}

fn audio_thread<T: Sample>(data: &mut [T], synth: &mut Synth) {
    synth.update_cmd();
    for channels in data.chunks_mut(2){
        let sample = synth.next_sample();
        channels[0] = Sample::from::<f32>(&sample.0);
        channels[1] = Sample::from::<f32>(&sample.1);
    }
}

struct Synth {
    rx: Receiver<SynthRegState>,
    reg_state: SynthRegState,
    n: u64,
    sample_rate: u64,
    timer_512_reset: u32,
    timer_512: u32,
    length_timer: u8,

    hz_frequency_1: u32,
    sound_length_1: u8,
    hz_frequency_2: u32,
    sound_length_2: u8,
}

impl Synth {
    fn new(rx: Receiver<SynthRegState>, cfg: &StreamConfig) -> Self {
        let sample_rate = cfg.sample_rate.0 as u64;
        Self {
            rx,
            reg_state: Default::default(),
            n: 0,
            sample_rate,
            timer_512_reset: (sample_rate / 512) as u32,
            timer_512: 0,
            length_timer: 0,
            hz_frequency_1: 0,
            sound_length_1: 0,
            hz_frequency_2: 0,
            sound_length_2: 0,
        }
    }

    fn update_cmd(&mut self) {
        let mut new_state = self.reg_state.clone();
        while let Ok(state) = self.rx.try_recv() {
            new_state = state;
        }
        if self.reg_state.sound_length_1 != new_state.sound_length_1{
            self.sound_length_1 = new_state.sound_length_1;
        }
        if self.reg_state.sound_length_2 != new_state.sound_length_2{
            self.sound_length_2 = new_state.sound_length_2;
        }
        self.hz_frequency_1 = (131072./(2048.-(new_state.frequency_1 as f32)).round()) as u32;
        self.hz_frequency_2 = (131072./(2048.-(new_state.frequency_2 as f32)).round()) as u32;
        self.reg_state = new_state;
    }

    fn next_sample(&mut self) -> (f32, f32) {
        if !self.reg_state.sound_enable {
            return (0.,0.);
        }
        self.timer_512 += 1;
        if self.timer_512 >= self.timer_512_reset {
            self.timer_512 = 0;
        }
        if self.timer_512 == 0 {
            self.length_timer += 1;
            if self.length_timer >= 2 {
                self.length_timer = 0;
            }
        }
        let square1 = self.next_square_1();
        let square2 = self.next_square_2();
        self.n += 1;
        if self.n % self.sample_rate == 0 {
            self.n = 0;
        }
        let mut left = 0.;
        let mut right = 0.;
        if self.reg_state.channel_pan & 0x10 != 0 {
            left += square1;
        }
        if self.reg_state.channel_pan & 0x20 != 0 {
            left += square2;
        }
        if self.reg_state.channel_pan & 0x01 != 0 {
            right += square1;
        }
        if self.reg_state.channel_pan & 0x02 != 0 {
            right += square2;
        }

        (left*0.1*self.reg_state.left_vol as f32 / 8., right*0.1 * self.reg_state.right_vol as f32 / 8.)
    }

    fn next_square_1(&mut self) -> f32 {
        if self.reg_state.length_en_1 && self.sound_length_1 != 0 && self.length_timer == 0 {
            self.sound_length_1 -= 1;
        }
        if self.reg_state.trigger_1 {
            self.sound_length_1 = self.reg_state.sound_length_1;
            self.reg_state.trigger_1 = false;
        }
        if self.sound_length_1 == 0 {
            return 0.;
        }
        let freq = self.hz_frequency_1 as u64;
        let normalized = (self.n * freq) % self.sample_rate;
        let cycle_index = (8. * (normalized as f32) / (self.sample_rate as f32)) as usize;
        SQUARE_PATTERN[self.reg_state.wave_pattern_1 as usize][cycle_index]
    }

    fn next_square_2(&mut self) -> f32 {
        return 0.;
        if self.reg_state.length_en_2 && self.sound_length_2 != 0 && self.length_timer == 0 {
            self.sound_length_2 -= 1;
        }
        if self.reg_state.trigger_2 {
            self.sound_length_2 = self.reg_state.sound_length_2;
            self.reg_state.trigger_2 = false;
        }
        if self.sound_length_2 == 0 {
            return 0.;
        }
        let freq =self.hz_frequency_2 as u64;
        let normalized = (self.n * freq) % self.sample_rate;
        let cycle_index = (8. * (normalized as f32) / (self.sample_rate as f32)) as usize;
        SQUARE_PATTERN[self.reg_state.wave_pattern_2 as usize][cycle_index]
    }
}

fn new_synth(cfg: &StreamConfig) -> (Synth, Sender<SynthRegState>) {
    let (tx, rx) = channel();
    (Synth::new(rx, cfg), tx)
}
