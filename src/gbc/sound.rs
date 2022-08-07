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

        #[cfg(feature = "audio-log")]
        println!("Supported audio config: {supported_config:?}");

        use cpal::SampleFormat;
        let sample_format = supported_config.sample_format();
        let min_bufsize = match supported_config.buffer_size() {
            &SupportedBufferSize::Range { min, max: _ } => min,
            _ => 0,
        };

        let mut config: StreamConfig = supported_config.into();
        config.buffer_size = BufferSize::Fixed((512).max(min_bufsize));

        #[cfg(feature = "audio-log")]
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
            0xff12 => {
                self.state.envelope_vol_1 << 4
                    | (self.state.envelope_increase_1 as u8) << 3
                    | self.state.envelope_sweep_1 & 0x7
            }
            _ => {
                #[cfg(feature = "audio-log")]
                eprintln!("Sound read at {addr:#x}");
                0
            }, // TODO: panic
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        #[cfg(feature = "audio-log")]
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
            0xff1a => {
                self.state.wave.enabled = value & 0x80 != 0;
            }
            0xff1b => {
                self.state.wave.length = 0x100 - value as u16;
            }
            0xff1c => {
                self.state.wave.volume_shift = match (value >> 5) & 0x3 {
                    0 => 4,
                    1 => 0,
                    2 => 1,
                    3 => 2,
                    _ => unreachable!(),
                };
            }
            0xff1d => {
                self.state.wave.frequency = self.state.wave.frequency & 0xff00 | value as u16;
            }
            0xff1e => {
                self.state.wave.frequency =
                    self.state.wave.frequency & 0x00ff | ((value & 0x7) as u16) << 8;
                self.state.wave.trigger = value & 0x80 != 0;
                self.state.wave.length_en = value & 0x40 != 0;
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
            0xff30..=0xff3f => {
                let index = (addr - 0xff30) as usize;
                self.state.wave.pattern[index * 2] = value >> 4;
                self.state.wave.pattern[index * 2 + 1] = value & 0x0f;
            }
            _ => {} // TODO: panic
        }
        self.tx
            .send(self.state.clone())
            .expect("Failed to send SyntCmd to audio thread");
        self.state.trigger_1 = false;
        self.state.trigger_2 = false;
        self.state.wave.trigger = false;
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

    wave: SynthWave,
}

#[derive(Default, Clone)]
struct SynthWave {
    enabled: bool,
    length: u16,
    volume_shift: u8,
    frequency: u16,
    length_en: bool,
    trigger: bool,
    pattern: [u8; 32],
}

const SQUARE_PATTERN: [[f32; 8]; 4] = [
    [-1., -1., -1., -1., -1., -1., -1., 1.],
    [-1., -1., -1., -1., -1., -1., 1., 1.],
    [-1., -1., -1., -1., 1., 1., 1., 1.],
    [1., 1., 1., 1., 1., 1., -1., -1.],
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
    for channels in data.chunks_mut(2) {
        let sample = synth.next_sample();
        channels[0] = Sample::from::<f32>(&sample.0);
        channels[1] = Sample::from::<f32>(&sample.1);
    }
}

struct Synth {
    rx: Receiver<SynthRegState>,
    reg_state: SynthRegState,
    n: u64,
    sample_rate: u32,
    timer_512_reset: u32,
    timer_512: u32,
    length_timer: u8,
    envelope_master_timer: u8,

    hz_frequency_1: u32,
    sound_length_1: u8,
    current_vol_1: u8,
    envelope_timer_1: u8,

    hz_frequency_2: u32,
    sound_length_2: u8,
    current_vol_2: u8,
    envelope_timer_2: u8,

    hz_frequency_3: u32,
    sound_length_3: u16,
    wave_timer: Timer,
    pattern_index_3: u32,
}

impl Synth {
    fn new(rx: Receiver<SynthRegState>, cfg: &StreamConfig) -> Self {
        let sample_rate = cfg.sample_rate.0;
        Self {
            rx,
            reg_state: Default::default(),
            n: 0,
            sample_rate,
            timer_512_reset: (sample_rate / 512) as u32,
            timer_512: 0,
            envelope_master_timer: 0,
            length_timer: 0,
            hz_frequency_1: 0,
            sound_length_1: 0,
            envelope_timer_1: 0,
            current_vol_1: 0,
            hz_frequency_2: 0,
            sound_length_2: 0,
            envelope_timer_2: 0,
            current_vol_2: 0,

            hz_frequency_3: 0,
            sound_length_3: 0,
            wave_timer: Timer::new(0, sample_rate),
            pattern_index_3: 0,
        }
    }

    fn update_cmd(&mut self) {
        let mut new_state = self.reg_state.clone();
        let mut trigger_1 = false;
        let mut trigger_2 = false;
        let mut trigger_3 = false;
        while let Ok(state) = self.rx.try_recv() {
            trigger_1 |= state.trigger_1;
            if state.trigger_1 {
                self.hz_frequency_1 =
                    (131072. / (2048. - (state.frequency_1 as f32)).round()) as u32;
                self.current_vol_1 = state.envelope_vol_1;
                self.envelope_timer_1 = state.envelope_sweep_1;
            }
            trigger_2 |= state.trigger_2;

            if state.trigger_2 {
                self.hz_frequency_2 =
                    (131072. / (2048. - (state.frequency_2 as f32)).round()) as u32;
                self.current_vol_2 = state.envelope_vol_2;
                self.envelope_timer_2 = state.envelope_sweep_2;
            }
            trigger_3 |= state.wave.trigger;
            if state.wave.trigger {
                self.hz_frequency_3 =
                    32 * (65536./ (2048. - (state.wave.frequency as f32)).round()) as u32;
                self.wave_timer = Timer::new(self.hz_frequency_3, self.sample_rate);
                self.pattern_index_3 = 2;
            }
            new_state = state;
        }
        new_state.trigger_1 = trigger_1;
        new_state.trigger_2 = trigger_2;
        new_state.wave.trigger = trigger_3;
        self.reg_state = new_state;
    }

    fn next_sample(&mut self) -> (f32, f32) {
        if !self.reg_state.sound_enable {
            return (0., 0.);
        }
        self.timer_512 += 1;
        if self.timer_512 >= self.timer_512_reset {
            self.timer_512 = 0;
        }
        if self.envelope_master_timer == 0 {
            self.envelope_master_timer = 1;
        }
        if self.length_timer == 0 {
            self.length_timer += 1;
        }
        if self.timer_512 == 0 {
            self.length_timer += 1;
            if self.length_timer >= 3 {
                // 0 only triggered for 1 sample
                self.length_timer = 0;
            }
            self.envelope_master_timer += 1;
            if self.envelope_master_timer >= 9 {
                // 0 only triggered for 1 sample
                self.envelope_master_timer = 0;
            }
        }
        let square1 = self.next_square_1();
        let square2 = self.next_square_2();
        let wave = self.next_wave();
        self.n += 1;
        if self.n > u64::MAX / 2 && self.n % (self.sample_rate as u64) == 0 {
            self.n = 0; // TODO: better with lcm ?
        }
        let mut left = 0.;
        let mut right = 0.;
        if self.reg_state.channel_pan & 0x10 != 0 {
            left += square1;
        }
        if self.reg_state.channel_pan & 0x20 != 0 {
            left += square2;
        }
        if self.reg_state.channel_pan & 0x40 != 0 {
            left += wave;
        }
        if self.reg_state.channel_pan & 0x01 != 0 {
            right += square1;
        }
        if self.reg_state.channel_pan & 0x02 != 0 {
            right += square2;
        }
        if self.reg_state.channel_pan & 0x04 != 0 {
            right += wave;
        }
        (
            left * 0.4 * self.reg_state.left_vol as f32 / 8.,
            right * 0.4 * self.reg_state.right_vol as f32 / 8.,
        )
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
        if self.reg_state.envelope_sweep_1 != 0 && self.envelope_master_timer == 0 {
            if self.envelope_timer_1 == 0 {
                self.envelope_timer_1 = self.reg_state.envelope_sweep_1;
                if self.reg_state.envelope_increase_1 && self.current_vol_1 != 0xf {
                    self.current_vol_1 += 1;
                } else if !self.reg_state.envelope_increase_1 && self.current_vol_1 != 0x0 {
                    self.current_vol_1 -= 1;
                }
            } else {
                self.envelope_timer_1 -= 1;
            }
        }
        let freq = self.hz_frequency_1 as u64;
        let normalized = (self.n * freq) % (self.sample_rate as u64);
        let cycle_index = (8. * (normalized as f32) / (self.sample_rate as f32)) as usize;
        SQUARE_PATTERN[self.reg_state.wave_pattern_1 as usize][cycle_index]
            * self.current_vol_1 as f32
            / 15.
    }

    fn next_square_2(&mut self) -> f32 {
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
        if self.reg_state.envelope_sweep_2 != 0 && self.envelope_master_timer == 0 {
            if self.envelope_timer_2 == 0 {
                self.envelope_timer_2 = self.reg_state.envelope_sweep_2;
                if self.reg_state.envelope_increase_2 && self.current_vol_2 != 0xf {
                    self.current_vol_2 += 1;
                } else if !self.reg_state.envelope_increase_2 && self.current_vol_2 != 0x0 {
                    self.current_vol_2 -= 1;
                }
            } else {
                self.envelope_timer_2 -= 1;
            }
        }
        let freq = self.hz_frequency_2 as u64;
        let normalized = (self.n * freq) % (self.sample_rate as u64);
        let cycle_index = (8. * (normalized as f32) / (self.sample_rate as f32)) as usize;
        SQUARE_PATTERN[self.reg_state.wave_pattern_2 as usize][cycle_index]
            * self.current_vol_2 as f32
            / 15.
    }

    fn next_wave(&mut self) -> f32 {
        if !self.reg_state.wave.enabled {
            return 0.;
        }
        if self.reg_state.wave.length_en && self.sound_length_3 != 0 && self.length_timer == 0 {
            self.sound_length_3 -= 1;
        }
        if self.reg_state.wave.trigger {
            self.sound_length_3 = self.reg_state.wave.length;
            self.reg_state.wave.trigger = false;
        }
        if self.sound_length_3 == 0 {
            return 0.;
        }
        self.wave_timer.sample_tick();
        if self.wave_timer.is_triggered() {
            self.pattern_index_3 += 1;
            if self.pattern_index_3 as usize >= self.reg_state.wave.pattern.len() {
                self.pattern_index_3 = 0;
            }
        }
        ((self.reg_state.wave.pattern[self.pattern_index_3 as usize]
            >> self.reg_state.wave.volume_shift) as f32)
            / 7.5 - 0.5
    }
}

fn new_synth(cfg: &StreamConfig) -> (Synth, Sender<SynthRegState>) {
    let (tx, rx) = channel();
    (Synth::new(rx, cfg), tx)
}

struct Timer {
    sample_period: f32,
    last_trigger: u32,
    sample_counter: u64,
    trigger: bool,
    enabled: bool,
}

impl Timer {
    fn new(hz_frequency: u32, sample_rate: u32) -> Self {
        if hz_frequency == 0 {
            return Self {
                sample_period: 0.,
                last_trigger: 0,
                sample_counter: 0,
                trigger: false,
                enabled: false,
            };
        }
        let sample_period = ((sample_rate as f64) / (hz_frequency as f64)) as f32;
        Self {
            sample_period,
            last_trigger: 0,
            sample_counter: 0,
            trigger: false,
            enabled: true,
        }
    }

    fn sample_tick(&mut self) {
        if !self.enabled {
            return;
        }
        self.sample_counter += 1;
        let approx_index = f32::trunc(self.sample_counter as f32 / self.sample_period) as u32;
        if approx_index != self.last_trigger {
            self.trigger = true;
            if self.sample_counter as f32 % self.sample_period < 0.001 {
                self.sample_counter = 0;
                self.last_trigger = 0;
            } else {
                self.last_trigger = approx_index;
            }
        } else {
            self.trigger = false;
        }
    }

    fn is_triggered(&self) -> bool {
        self.trigger
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wave_timer() {
        let sample_rate = 51200;
        let freq = 440;
        let mut timer = Timer::new(freq, sample_rate);
        let mut timer_count = 0;
        for _ in 0..(sample_rate * 2) {
            timer.sample_tick();
            if timer.is_triggered() {
                timer_count += 1;
            }
        }
        assert_eq!(timer_count, freq * 2);
    }
    #[test]
    fn wave_timer_lowfreq() {
        let sample_rate = 51200;
        let freq = 100;
        let mut timer = Timer::new(freq, sample_rate);
        let mut timer_count = 0;
        for _ in 0..(sample_rate * 2) {
            timer.sample_tick();
            if timer.is_triggered() {
                timer_count += 1;
            }
        }
        assert_eq!(timer_count, freq * 2);
    }
}