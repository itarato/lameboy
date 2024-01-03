use std::sync::Arc;
use std::sync::Mutex;

use sdl2::audio::AudioCallback;
use sdl2::audio::AudioDevice;
use sdl2::audio::AudioSpecDesired;

use crate::conf::*;
use crate::util::*;

#[derive(Debug)]
struct SoundPacket {
    is_on: bool,
    pitch: f32,                   // 1.0 .. ~k
    volume: f32,                  // 0.0 .. 1.0
    envelope_sweep_length: usize, // 22050 = 1s
    envelope_direction_down: bool,
    waveform: f32, // 0.0 .. 1.0
    restart: bool,
    length_enable: bool,
    length: u8,
    length_counter: Counter,
}

impl SoundPacket {
    fn new() -> SoundPacket {
        SoundPacket {
            is_on: false,
            pitch: 0.0,
            volume: 0.0,
            envelope_sweep_length: 0,
            envelope_direction_down: true,
            waveform: 0.0,
            restart: false,
            length_enable: false,
            length: 0,
            length_counter: Counter::new(CPU_HZ as u64 / 256),
        }
    }

    fn tick(&mut self, cycles: u64) {
        if self.length_enable && self.length > 0 {
            if self.length_counter.tick_and_check_overflow(cycles) {
                self.length -= 1;

                if self.length == 0 {
                    self.is_on = false;
                }
            }
        }
    }
}

struct SquareWave {
    freq: f32,
    phase: f32,
    pocket: Arc<Mutex<SoundPacket>>,
    envelope_sweep_counter: usize,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        let mut pocket = self.pocket.lock().expect("Cannot lock pocket");

        for x in out.iter_mut() {
            if !pocket.is_on {
                *x = 0.0;
                continue;
            }

            if (*pocket).restart {
                (*pocket).restart = false;
                self.envelope_sweep_counter = (*pocket).envelope_sweep_length;
            }

            let pitch = pocket.pitch;
            self.phase = (self.phase + (pitch / self.freq)) % 1.0;

            if (*pocket).envelope_sweep_length > 0 {
                if self.envelope_sweep_counter > 0 {
                    self.envelope_sweep_counter -= 1;
                } else {
                    (*pocket).volume += if (*pocket).envelope_direction_down {
                        -1f32 / 15f32
                    } else {
                        1f32 / 15f32
                    };
                    self.envelope_sweep_counter = (*pocket).envelope_sweep_length;
                }
            }

            if (*pocket).volume < 0f32 {
                (*pocket).volume = 0.0;
            } else if (*pocket).volume > 1f32 {
                (*pocket).volume = 1.0;
            }

            *x = if self.phase <= pocket.waveform {
                pocket.volume
            } else {
                -pocket.volume
            }
        }
    }
}

pub struct Apu {
    disable_sound: bool,

    nr10: u8,
    nr11: u8,
    nr12: u8,
    nr13: u8,
    nr14: u8,
    nr21: u8,
    nr22: u8,
    nr23: u8,
    nr24: u8,
    nr30: u8,
    nr31: u8,
    nr32: u8,
    nr33: u8,
    nr34: u8,
    nr41: u8,
    nr42: u8,
    nr43: u8,
    nr44: u8,
    nr50: u8,
    nr51: u8,
    nr52: u8,

    _channel_1_device: AudioDevice<SquareWave>,
    channel_1_out: Arc<Mutex<SoundPacket>>,

    _channel_2_device: AudioDevice<SquareWave>,
    channel_2_out: Arc<Mutex<SoundPacket>>,
}

impl Apu {
    pub fn new(disable_sound: bool) -> Self {
        let sdl_context = sdl2::init().unwrap();

        let desired_spec = AudioSpecDesired {
            freq: Some(44_100),
            channels: Some(1),
            samples: None,
        };

        let channel_1_out = Arc::new(Mutex::new(SoundPacket::new()));
        let _channel_1_device = sdl_context
            .audio()
            .unwrap()
            .open_playback(None, &desired_spec, |spec| SquareWave {
                freq: spec.freq as f32,
                phase: 0.5,
                pocket: channel_1_out.clone(),
                envelope_sweep_counter: 0,
            })
            .unwrap();
        _channel_1_device.resume();

        let channel_2_out = Arc::new(Mutex::new(SoundPacket::new()));
        let _channel_2_device = sdl_context
            .audio()
            .unwrap()
            .open_playback(None, &desired_spec, |spec| SquareWave {
                freq: spec.freq as f32,
                phase: 0.5,
                pocket: channel_2_out.clone(),
                envelope_sweep_counter: 0,
            })
            .unwrap();
        _channel_2_device.resume();

        Apu {
            nr10: 0,
            nr11: 0,
            nr12: 0,
            nr13: 0,
            nr14: 0,
            nr21: 0,
            nr22: 0,
            nr23: 0,
            nr24: 0,
            nr30: 0,
            nr31: 0,
            nr32: 0,
            nr33: 0,
            nr34: 0,
            nr41: 0,
            nr42: 0,
            nr43: 0,
            nr44: 0,
            nr50: 0,
            nr51: 0,
            nr52: 0,
            _channel_1_device,
            channel_1_out,
            _channel_2_device,
            channel_2_out,
            disable_sound,
        }
    }

    pub fn update(&mut self, cycles: u64) {
        self.channel_1_out.lock().unwrap().tick(cycles);
        self.channel_2_out.lock().unwrap().tick(cycles);
    }

    pub fn write(&mut self, loc: u16, byte: u8) {
        match loc {
            MEM_LOC_NR10 => self.nr10 = byte,
            // NR11: Channel 1 length timer & duty cycle
            MEM_LOC_NR11 => self.nr11 = byte,
            // NR12: Channel 1 volume & envelope
            MEM_LOC_NR12 => self.nr12 = byte,
            // NR13: Channel 1 period low [write-only].
            MEM_LOC_NR13 => self.nr13 = byte,
            // FF14 — NR14: Channel 1 period high & control.
            MEM_LOC_NR14 => {
                self.nr14 = byte;
                self.channel1_update();
            }

            MEM_LOC_NR21 => self.nr21 = byte,
            MEM_LOC_NR22 => self.nr22 = byte,
            MEM_LOC_NR23 => self.nr23 = byte,
            MEM_LOC_NR24 => {
                self.nr24 = byte;
                self.channel2_update();
            }

            MEM_LOC_NR30 => self.nr30 = byte,
            MEM_LOC_NR31 => self.nr31 = byte,
            MEM_LOC_NR32 => self.nr32 = byte,
            MEM_LOC_NR33 => self.nr33 = byte,
            MEM_LOC_NR34 => self.nr34 = byte,

            MEM_LOC_NR41 => self.nr41 = byte,
            MEM_LOC_NR42 => self.nr42 = byte,
            MEM_LOC_NR43 => self.nr43 = byte,
            MEM_LOC_NR44 => self.nr44 = byte,

            // FF24 — NR50: Master volume & VIN panning
            MEM_LOC_NR50 => self.nr50 = byte,
            // FF25 — NR51: Apu panning
            MEM_LOC_NR51 => self.nr51 = byte,
            // FF26 — NR52: Audio master control
            MEM_LOC_NR52 => {
                // Cannot manually set CHx enable/disable flags.
                assert!(byte & 0b1111 == 0);
                self.nr52 = byte;
            }
            _ => unimplemented!("Apu chip loc write: {:#06X} not implemented", loc),
        };
    }

    pub fn read(&self, loc: u16) -> Result<u8, Error> {
        Err(format!("Apu chip read not implemented: {:#06X}", loc).into())
    }

    fn audio_on(&self) -> bool {
        is_bit(self.nr52, 7)
    }

    fn channel1_update(&self) {
        if self.disable_sound || !self.audio_on() || !is_bit(self.nr14, 7) {
            return;
        }

        set_bit(self.nr52, 0, true);

        // 00: 12.5%
        // 01: 25%
        // 10: 50%
        // 11: 75%
        let wave_duty = self.nr11 >> 6;
        // When the length timer reaches 64, the channel is turned off: nr52 bit-0 + nr14 bit-7 -> 0.
        let init_length_timer = self.nr11 & 0b11_1111;

        let init_volume = self.nr12 >> 4;
        let is_envelope_direction_increase = is_bit(self.nr12, 3);
        let sweep_pace = self.nr12 & 0b111;

        let length_enable = is_bit(self.nr14, 6);
        let period_hi = (self.nr14 & 0b111) as u16;
        let period_lo = self.nr13 as u16;
        let period = (period_hi << 8) | period_lo;

        let out_freq = (CPU_HZ as f32 / 32.0) / (2048.0 - period as f32);
        let out_volume = init_volume as f32 / 15.0;
        let out_envelop_sweep_length = (44_100 * sweep_pace as usize) / 64;
        let out_waveform = match wave_duty {
            0b00 => 0.125,
            0b01 => 0.25,
            0b10 => 0.5,
            0b11 => 0.75,
            _ => panic!("Illegal wave form"),
        };

        {
            let mut pocket = self.channel_1_out.lock().unwrap();
            pocket.is_on = true;
            pocket.pitch = out_freq;
            pocket.volume = out_volume;
            pocket.envelope_sweep_length = out_envelop_sweep_length;
            pocket.envelope_direction_down = !is_envelope_direction_increase;
            pocket.waveform = out_waveform;
            pocket.length_enable = length_enable;
            pocket.length = init_length_timer;
        }
    }

    fn channel2_update(&self) {
        if self.disable_sound || !self.audio_on() || !is_bit(self.nr24, 7) {
            return;
        }

        set_bit(self.nr52, 1, true);

        // 00: 12.5%
        // 01: 25%
        // 10: 50%
        // 11: 75%
        let wave_duty = self.nr21 >> 6;
        // When the length timer reaches 64, the channel is turned off: nr52 bit-0 + nr14 bit-7 -> 0.
        let init_length_timer = self.nr21 & 0b11_1111;

        let init_volume = self.nr22 >> 4;
        let is_envelope_direction_increase = is_bit(self.nr22, 3);
        let sweep_pace = self.nr22 & 0b111;

        let length_enable = is_bit(self.nr24, 6);
        let period_hi = (self.nr24 & 0b111) as u16;
        let period_lo = self.nr23 as u16;
        let period = (period_hi << 8) | period_lo;

        let out_freq = (CPU_HZ as f32 / 32.0) / (2048.0 - period as f32);
        let out_volume = init_volume as f32 / 15.0;
        let out_envelop_sweep_length = (44_100 * sweep_pace as usize) / 64;
        let out_waveform = match wave_duty {
            0b00 => 0.125,
            0b01 => 0.25,
            0b10 => 0.5,
            0b11 => 0.75,
            _ => panic!("Illegal wave form"),
        };

        {
            let mut pocket = self.channel_2_out.lock().unwrap();
            pocket.is_on = true;
            pocket.pitch = out_freq;
            pocket.volume = out_volume;
            pocket.envelope_sweep_length = out_envelop_sweep_length;
            pocket.envelope_direction_down = !is_envelope_direction_increase;
            pocket.waveform = out_waveform;
            pocket.length_enable = length_enable;
            pocket.length = init_length_timer;
        }
    }
}
