use std::sync::Arc;
use std::sync::Mutex;

use sdl2::audio::AudioCallback;
use sdl2::audio::AudioDevice;
use sdl2::audio::AudioFormatNum;
use sdl2::audio::AudioSpecDesired;

use crate::conf::*;
use crate::util::*;

const NOISE_CHANNEL_DIVISORS: [u8; 8] = [8, 16, 32, 48, 64, 80, 96, 112];

struct NoiseSoundPacket {
    active: bool,
    length: u8,
    length_enable: bool,
    volume: f32,
    freq: f32,
    is_envelope_dir_inc: bool,
    envelope_sweep_length: u32,
    lfsr_short_mode: bool,
    lfsr: u16,
    speaker_left: bool,
    speaker_right: bool,
    global_volume_left: f32,
    global_volume_right: f32,
}

impl NoiseSoundPacket {
    fn new() -> NoiseSoundPacket {
        NoiseSoundPacket {
            active: false,
            length: 0,
            length_enable: false,
            volume: 0.0,
            freq: 440.0,
            is_envelope_dir_inc: false,
            envelope_sweep_length: 0,
            lfsr_short_mode: false,
            lfsr: 0,
            speaker_left: true,
            speaker_right: true,
            global_volume_left: 0.5,
            global_volume_right: 0.5,
        }
    }

    fn tick(&mut self, clock_overflow: bool) {
        if clock_overflow && self.length_enable && self.length > 0 {
            self.length -= 1;
            self.active = self.length > 0;
        }
    }
}

struct PulseChannel {
    device_freq: f32,
    phase: f32,
    envelope_sweep_counter: usize,
    active: bool,
    freq: f32,   // 1.0 .. ~k
    volume: f32, // 0.0 .. 1.0
    sweep_counter: Option<Counter>,
    sweep_direction_sub: bool,
    sweep_step: u8,
    envelope_sweep_length: usize, // 22050 = 1s
    envelope_direction_down: bool,
    waveform: f32, // 0.0 .. 1.0
    length_enable: bool,
    length: u8,
    speaker_left: bool,
    speaker_right: bool,
    global_volume_left: f32,
    global_volume_right: f32,
    period: u16,
}

impl PulseChannel {
    fn new(device_freq: f32) -> PulseChannel {
        PulseChannel {
            device_freq,
            phase: 0.0,
            envelope_sweep_counter: 0,
            active: false,
            freq: 0.0,
            volume: 0.0,
            sweep_counter: None,
            sweep_direction_sub: true,
            sweep_step: 0,
            envelope_sweep_length: 0,
            envelope_direction_down: true,
            waveform: 0.0,
            length_enable: false,
            length: 0,
            speaker_left: true,
            speaker_right: true,
            global_volume_left: 0.5,
            global_volume_right: 0.5,
            period: 0,
        }
    }

    #[must_use]
    fn tick(&mut self, clock_overflow: bool, cpu_clocks: u32) -> Option<u16> {
        if clock_overflow && self.length_enable && self.length > 0 {
            self.length -= 1;
            self.active = self.length > 0;
        }

        if let Some(sweep_counter) = self.sweep_counter.as_mut() {
            sweep_counter.tick(cpu_clocks);
            let mut overflow_count = sweep_counter.check_overflow_count();
            let old_period = self.period;

            while overflow_count > 0 {
                overflow_count -= 1;

                let dir: i16 = if self.sweep_direction_sub { -1 } else { 1 };
                let new_period =
                    self.period as i16 + dir * (self.period as i16 / (1 << self.sweep_step));

                let new_period: u16 = if new_period < 0 {
                    0
                } else if new_period > 0b0000_0111_1111_1111 {
                    self.sweep_counter = None;
                    0b0000_0111_1111_1111
                } else {
                    new_period as u16
                };
                self.period = new_period;
                self.freq = (CPU_HZ as f32 / 32.0) / (2048.0 - self.period as f32);
            }

            if old_period == self.period {
                None
            } else {
                Some(self.period)
            }
        } else {
            None
        }
    }

    fn generate(&mut self, out: &mut [f32], volume_divider: f32) {
        if !self.speaker_left && !self.speaker_right {
            return;
        }

        for chunk in out.chunks_exact_mut(2) {
            let value = if !self.active {
                0.0
            } else {
                if self.envelope_sweep_length > 0 {
                    if self.envelope_sweep_counter > 0 {
                        self.envelope_sweep_counter -= 1;
                    } else {
                        self.volume += if self.envelope_direction_down {
                            -1f32 / 15f32
                        } else {
                            1f32 / 15f32
                        };
                        self.envelope_sweep_counter = self.envelope_sweep_length;
                    }
                }

                if self.volume < 0f32 {
                    self.volume = 0.0;
                } else if self.volume > 1f32 {
                    self.volume = 1.0;
                }

                self.phase = (self.phase + (self.freq / self.device_freq)) % 1.0;
                if self.phase <= self.waveform {
                    self.volume
                } else {
                    -self.volume
                }
            };

            if self.speaker_left {
                chunk[0] += (value / volume_divider) * self.global_volume_left;
                // Left speaker.
            }
            if self.speaker_right {
                chunk[1] += (value / volume_divider) * self.global_volume_right;
                // Right speaker.
            }
        }
    }
}

struct WaveChannel {
    freq: f32,
    phase: f32,
    active: bool,
    length: u8,
    out_level: u8,
    length_enable: bool,
    wave_pattern: [u8; 16],
    tone_freq: f32,
    speaker_left: bool,
    speaker_right: bool,
    global_volume_left: f32,
    global_volume_right: f32,
}

impl WaveChannel {
    fn new(freq: f32) -> WaveChannel {
        WaveChannel {
            freq,
            phase: 0.0,
            active: false,
            length: 0,
            out_level: 0,
            length_enable: false,
            wave_pattern: [0; 16],
            tone_freq: 0.0,
            speaker_left: true,
            speaker_right: true,
            global_volume_left: 0.5,
            global_volume_right: 0.5,
        }
    }

    fn tick(&mut self, clock_overflow: bool) {
        if clock_overflow && self.length_enable && self.length > 0 {
            self.length -= 1;
            self.active = self.length > 0;
        }
    }

    fn generate(&mut self, out: &mut [f32], volume_divider: f32) {
        if !self.speaker_left && !self.speaker_right {
            return;
        }

        for chunk in out.chunks_exact_mut(2) {
            let value = if !self.active {
                0.0
            } else {
                self.phase = (self.phase + (self.tone_freq / self.freq)) % 1.0;

                let half_adjusted_phase = if self.phase < 0.5 {
                    self.phase
                } else {
                    self.phase - 0.5
                };
                let wave_ptr = (half_adjusted_phase / (0.5 / 31.999)) as usize;

                let wave_sample = if wave_ptr % 2 == 0 {
                    self.wave_pattern[wave_ptr / 2] >> 4
                } else {
                    self.wave_pattern[wave_ptr / 2] & 0xF
                } >> self.out_level;

                if self.phase <= 0.5 {
                    wave_sample as f32 / 15.0
                } else {
                    -(wave_sample as f32 / 15.0)
                }
            };

            if self.speaker_left {
                chunk[0] += (value / volume_divider) * self.global_volume_left;
                // Left speaker.
            }
            if self.speaker_right {
                chunk[1] += (value / volume_divider) * self.global_volume_right;
                // Right speaker.
            }
        }
    }
}

struct NoiseChannel {
    freq: f32,
    phase: f32,
    envelope_sweep_counter: u32,
    packet: Arc<Mutex<NoiseSoundPacket>>,
    prev_lfsr_div: u32,
}

impl NoiseChannel {
    fn generate(&mut self, out: &mut [f32], volume_divider: f32) {
        let mut packet = self.packet.lock().expect("Cannot lock pocket");

        if !packet.speaker_left && !packet.speaker_right {
            return;
        }

        // This is a hack but that's what most the DMG sounds like :/
        const LFSR_FREQ: f32 = 60.0;

        for chunk in out.chunks_exact_mut(2) {
            let value = if !packet.active {
                0.0
            } else {
                if packet.envelope_sweep_length > 0 {
                    if self.envelope_sweep_counter > 0 {
                        self.envelope_sweep_counter -= 1;
                    } else {
                        (*packet).volume += if !packet.is_envelope_dir_inc {
                            -1f32 / 15f32
                        } else {
                            1f32 / 15f32
                        };
                        self.envelope_sweep_counter = packet.envelope_sweep_length;

                        if (*packet).volume < 0f32 {
                            packet.volume = 0.0;
                        } else if (*packet).volume > 1f32 {
                            packet.volume = 1.0;
                        }
                    }
                }

                self.phase = (self.phase + (packet.freq / self.freq)) % 1.0;
                let lfsr_div = (self.phase / (1.0 / LFSR_FREQ)) as u32;
                if lfsr_div != self.prev_lfsr_div {
                    self.prev_lfsr_div = lfsr_div;

                    let new_bit = !(packet.lfsr ^ (packet.lfsr >> 1)) & 0b1;
                    packet.lfsr = (packet.lfsr >> 1) | new_bit << 14;

                    if packet.lfsr_short_mode {
                        packet.lfsr = set_bit_16(packet.lfsr, 7, new_bit != 0);
                    }
                }

                if self.phase <= 0.5 {
                    packet.volume * ((packet.lfsr >> 14) & 0b1) as f32
                } else {
                    -packet.volume * ((packet.lfsr >> 14) & 0b1) as f32
                }
            };

            if packet.speaker_left {
                chunk[0] += (value / volume_divider) * packet.global_volume_left;
                // Left speaker.
            }
            if packet.speaker_right {
                chunk[1] += (value / volume_divider) * packet.global_volume_right;
                // Right speaker.
            }
        }
    }
}

struct DmgChannels {
    ch1_pulse: PulseChannel,
    ch2_pulse: PulseChannel,
    ch3_wave: WaveChannel,
    ch4_noise: NoiseChannel,
}

impl DmgChannels {
    fn new(freq: f32, ch4_packet: Arc<Mutex<NoiseSoundPacket>>) -> DmgChannels {
        DmgChannels {
            ch1_pulse: PulseChannel::new(freq),
            ch2_pulse: PulseChannel::new(freq),
            ch3_wave: WaveChannel::new(freq),
            ch4_noise: NoiseChannel {
                freq,
                phase: 0.0,
                envelope_sweep_counter: 0,
                packet: ch4_packet,
                prev_lfsr_div: 0,
            },
        }
    }
}

impl AudioCallback for DmgChannels {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // MUST BE EQUAL TO HOW MANY PARTS CONTRIBUTING TO THE DEVICE.
        const PARTS_LEN: f32 = 4.0;

        // Silence it out - so channels can _add_ their part.
        out.iter_mut().for_each(|b| *b = Self::Channel::SILENCE);

        self.ch1_pulse.generate(out, PARTS_LEN);
        self.ch2_pulse.generate(out, PARTS_LEN);
        self.ch3_wave.generate(out, PARTS_LEN);
        self.ch4_noise.generate(out, PARTS_LEN);
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

    wave_pattern_ram: [u8; 16],

    sound_device: AudioDevice<DmgChannels>,
    ch4_packet: Arc<Mutex<NoiseSoundPacket>>,

    clock: Counter,
}

impl Apu {
    pub fn new(disable_sound: bool) -> Self {
        let sdl_context = sdl2::init().unwrap();

        let desired_spec = AudioSpecDesired {
            freq: Some(44_100),
            channels: Some(2),
            samples: Some(256),
        };

        let ch4_packet = Arc::new(Mutex::new(NoiseSoundPacket::new()));

        let sound_device = sdl_context
            .audio()
            .unwrap()
            .open_playback(None, &desired_spec, |spec| {
                DmgChannels::new(spec.freq as _, ch4_packet.clone())
            })
            .unwrap();
        sound_device.resume();

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
            wave_pattern_ram: [0; 16],
            // Just to keep the thread alive.
            sound_device,
            ch4_packet,
            disable_sound,
            clock: Counter::new(CPU_HZ / 256),
        }
    }

    pub fn update(&mut self, cpu_clocks: u32) {
        // TODO: Put this ticker entirely into the sound device.
        let clock_overflow = self.clock.tick_and_check_overflow(cpu_clocks);

        {
            let ref mut ch1 = self.sound_device.lock().ch1_pulse;
            if let Some(new_period) = ch1.tick(clock_overflow, cpu_clocks) {
                self.nr13 = (new_period & 0xff) as u8;
                self.nr14 = (self.nr14 & !0b111) | ((new_period >> 8) & 0b111) as u8;
            }
        }

        let _ = self
            .sound_device
            .lock()
            .ch2_pulse
            .tick(clock_overflow, cpu_clocks);

        self.sound_device.lock().ch3_wave.tick(clock_overflow);

        self.ch4_packet.lock().unwrap().tick(clock_overflow);
    }

    pub fn write(&mut self, loc: u16, byte: u8) {
        match loc {
            // TODO: Maybe we need this:
            // "During the All Sound OFF mode, each sound mode register cannot be set.)"
            MEM_LOC_NR10 => self.nr10 = byte,
            // NR11: Channel 1 length timer & duty cycle
            MEM_LOC_NR11 => {
                self.nr11 = byte;

                let length = 64 - (self.nr11 & 0b11_1111);
                self.sound_device.lock().ch1_pulse.length = length;
            }
            // NR12: Channel 1 volume & envelope
            MEM_LOC_NR12 => self.nr12 = byte,
            // NR13: Channel 1 period low [write-only].
            MEM_LOC_NR13 => self.nr13 = byte,
            // FF14 — NR14: Channel 1 period high & control.
            MEM_LOC_NR14 => {
                self.nr14 = byte;
                self.channel1_update();
            }

            MEM_LOC_NR21 => {
                self.nr21 = byte;

                let length = 64 - (self.nr21 & 0b11_1111);
                self.sound_device.lock().ch2_pulse.length = length;
            }
            MEM_LOC_NR22 => self.nr22 = byte,
            MEM_LOC_NR23 => self.nr23 = byte,
            MEM_LOC_NR24 => {
                self.nr24 = byte;
                self.channel2_update();
            }

            MEM_LOC_NR30 => {
                self.nr30 = byte;

                let dac_on = is_bit(self.nr30, 7);
                if !dac_on {
                    self.sound_device.lock().ch3_wave.active = false;
                }
            }
            MEM_LOC_NR31 => {
                self.nr31 = byte;

                let length = 255 - self.nr31;
                self.sound_device.lock().ch3_wave.length = length;
            }
            MEM_LOC_NR32 => self.nr32 = byte,
            MEM_LOC_NR33 => self.nr33 = byte,
            MEM_LOC_NR34 => {
                self.nr34 = byte;
                self.channel3_update();
            }

            MEM_LOC_NR41 => {
                self.nr41 = byte;

                let length = 64 - (self.nr41 & 0b11_1111);
                self.ch4_packet.lock().unwrap().length = length;
            }
            MEM_LOC_NR42 => self.nr42 = byte,
            MEM_LOC_NR43 => self.nr43 = byte,
            MEM_LOC_NR44 => {
                self.nr44 = byte;
                self.channel4_update();
            }

            // FF24 — NR50: Master volume & VIN panning
            MEM_LOC_NR50 => {
                self.nr50 = byte;

                let volume_left_bits = (self.nr50 >> 4) & 0b111;
                let volume_right_bits = self.nr50 & 0b111;

                let volume_left = 8.0 / (volume_left_bits + 1) as f32;
                let volume_right = 8.0 / (volume_right_bits + 1) as f32;

                {
                    let ref mut packet = self.sound_device.lock().ch1_pulse;
                    packet.global_volume_left = volume_left;
                    packet.global_volume_right = volume_right;
                }
                {
                    let ref mut packet = self.sound_device.lock().ch2_pulse;
                    packet.global_volume_left = volume_left;
                    packet.global_volume_right = volume_right;
                }
                {
                    let ref mut packet = self.sound_device.lock().ch3_wave;
                    packet.global_volume_left = volume_left;
                    packet.global_volume_right = volume_right;
                }
                {
                    let mut packet = self.ch4_packet.lock().unwrap();
                    packet.global_volume_left = volume_left;
                    packet.global_volume_right = volume_right;
                }
            }
            // FF25 — NR51: Apu panning
            MEM_LOC_NR51 => {
                self.nr51 = byte;
            }
            // FF26 — NR52: Audio master control
            MEM_LOC_NR52 => {
                // Cannot manually set CHx enable/disable flags.
                self.nr52 = byte & 0xF0;

                if !self.audio_on() {
                    self.sound_device.lock().ch1_pulse.active = false;
                    self.sound_device.lock().ch2_pulse.active = false;
                    self.sound_device.lock().ch3_wave.active = false;
                    self.ch4_packet.lock().unwrap().active = false;
                }
            }
            MEM_LOC_WAVE_PATTERN_START..=MEM_LOC_WAVE_PATTERN_END => {
                if !self.is_ch3_on() {
                    self.wave_pattern_ram[(loc - MEM_LOC_WAVE_PATTERN_START) as usize] = byte;
                } else {
                    log::error!("Write to CH3 wave patterns while on: {:04X}", loc);
                    // Make sure the turn-off mechanism works. If it is - error can be ignored.
                }
            }
            _ => unimplemented!("Apu chip loc write: {:#06X} not implemented", loc),
        };
    }

    pub fn read(&mut self, loc: u16) -> Result<u8, Error> {
        match loc {
            MEM_LOC_NR50 => Ok(self.nr50),
            MEM_LOC_NR51 => Ok(self.nr51),
            MEM_LOC_NR52 => {
                let mut byte = self.nr52 & 0xF0;

                byte = set_bit(byte, 0, self.sound_device.lock().ch1_pulse.active);
                byte = set_bit(byte, 1, self.sound_device.lock().ch2_pulse.active);
                byte = set_bit(byte, 2, self.sound_device.lock().ch3_wave.active);
                byte = set_bit(byte, 3, self.ch4_packet.lock().unwrap().active);

                Ok(byte)
            }
            MEM_LOC_WAVE_PATTERN_START..=MEM_LOC_WAVE_PATTERN_END => {
                if self.is_ch3_on() {
                    Ok(self.wave_pattern_ram[(loc - MEM_LOC_WAVE_PATTERN_START) as usize])
                } else {
                    // NOT SURE: this might return the actual bytes.
                    // Strange quote: "Wave RAM can be accessed normally even if the DAC is on, as long as the channel is not active."
                    // -> how else wave channel can be not active then dac-off?
                    Ok(0xFF)
                }
            }
            _ => Err(format!("Apu chip read not implemented: {:#06X}", loc).into()),
        }
    }

    fn audio_on(&self) -> bool {
        is_bit(self.nr52, 7)
    }

    fn is_ch3_on(&mut self) -> bool {
        self.sound_device.lock().ch3_wave.active
    }

    fn channel1_update(&mut self) {
        let length_enable = is_bit(self.nr14, 6);

        if self.disable_sound || !self.audio_on() || !is_bit(self.nr14, 7) {
            self.sound_device.lock().ch1_pulse.length_enable = length_enable;
            return;
        }

        let pace = (self.nr10 >> 4) & 0b111;
        let sweep_direction_sub = is_bit(self.nr10, 3);
        let sweep_individual_step = self.nr10 & 0b111;

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

        let period_hi = (self.nr14 & 0b111) as u16;
        let period_lo = self.nr13 as u16;
        let period = (period_hi << 8) | period_lo;

        let out_freq = (CPU_HZ as f32 / 32.0) / (2048.0 - period as f32);
        let out_volume = init_volume as f32 / 15.0;
        let out_sweep_counter = match pace {
            0 => None,
            v => Some(Counter::new(CPU_HZ / (128 * v as u32))),
        };
        let out_envelop_sweep_length = (44_100 * sweep_pace as usize) / 64;
        let out_waveform = match wave_duty {
            0b00 => 0.125,
            0b01 => 0.25,
            0b10 => 0.5,
            0b11 => 0.75,
            _ => panic!("Illegal wave form"),
        };
        let active = init_volume > 0 || is_envelope_direction_increase;

        {
            let is_ch1_left = self.is_ch1_left();
            let is_ch1_right = self.is_ch1_right();

            let ref mut packet = self.sound_device.lock().ch1_pulse;
            packet.active = active;
            packet.freq = out_freq;
            packet.volume = out_volume;
            packet.sweep_direction_sub = sweep_direction_sub;
            packet.sweep_step = sweep_individual_step;
            packet.sweep_counter = out_sweep_counter;
            packet.envelope_sweep_length = out_envelop_sweep_length;
            packet.envelope_direction_down = !is_envelope_direction_increase;
            packet.waveform = out_waveform;
            packet.length_enable = length_enable;
            packet.length = init_length_timer;
            packet.speaker_left = is_ch1_left;
            packet.speaker_right = is_ch1_right;
            packet.period = period;
        }
    }

    fn channel2_update(&mut self) {
        let length_enable = is_bit(self.nr24, 6);

        if self.disable_sound || !self.audio_on() || !is_bit(self.nr24, 7) {
            self.sound_device.lock().ch2_pulse.length_enable = length_enable;
            return;
        }

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

        let active = init_volume > 0 || is_envelope_direction_increase;

        {
            let is_ch2_left = self.is_ch2_left();
            let is_ch2_right = self.is_ch2_right();

            let ref mut packet = self.sound_device.lock().ch2_pulse;
            packet.active = active;
            packet.freq = out_freq;
            packet.volume = out_volume;
            packet.envelope_sweep_length = out_envelop_sweep_length;
            packet.envelope_direction_down = !is_envelope_direction_increase;
            packet.waveform = out_waveform;
            packet.length_enable = length_enable;
            packet.length = init_length_timer;
            packet.speaker_left = is_ch2_left;
            packet.speaker_right = is_ch2_right;
        }
    }

    fn channel3_update(&mut self) {
        let length_enable = is_bit(self.nr34, 6);

        if self.disable_sound || !self.audio_on() || !is_bit(self.nr34, 7) {
            self.sound_device.lock().ch3_wave.length_enable = length_enable;
            return;
        }

        let dac_on = is_bit(self.nr30, 7);
        // The higher the length timer, the shorter the time before the channel is cut.
        let init_length_timer = self.nr31;
        // 00	Mute (No sound)
        // 01	100% volume (use samples read from Wave RAM as-is)
        // 10	50% volume (shift samples read from Wave RAM right once)
        // 11	25% volume (shift samples read from Wave RAM right twice)
        let output_level_raw = (self.nr32 >> 5) & 0b11;
        let period_lo = self.nr33 as u16;
        let period_hi = (self.nr34 & 0b111) as u16;
        let period = (period_hi << 8) | period_lo;
        // let length_enable = is_bit(self.nr34, 6);
        let wave_pattern = self.wave_pattern_ram.clone();

        let tone_freq = (2097152.0 / (0x800 - period) as f32) / 32.0;
        let out_level: u8 = match output_level_raw {
            0 => 4,
            1 => 0,
            2 => 1,
            3 => 2,
            _ => unreachable!(),
        };
        let length = 0xff - init_length_timer;
        let active = dac_on;

        {
            let is_ch3_left = self.is_ch3_left();
            let is_ch3_right = self.is_ch3_right();

            let ref mut packet = self.sound_device.lock().ch3_wave;

            packet.active = active;
            packet.tone_freq = tone_freq;
            packet.length = length;
            packet.out_level = out_level;
            // Not sure if this should always be true - but for now it is. Otherwise this goes on beeping forever.
            packet.length_enable = true;
            packet.wave_pattern = wave_pattern;
            packet.speaker_left = is_ch3_left;
            packet.speaker_right = is_ch3_right;
        }
    }

    fn channel4_update(&mut self) {
        let length_enable = is_bit(self.nr44, 6);

        if self.disable_sound || !self.audio_on() || !is_bit(self.nr44, 7) {
            self.ch4_packet.lock().unwrap().length_enable = length_enable;
            return;
        }

        let initial_length_timer = self.nr41 & 0b11_1111;

        let init_volume = self.nr42 >> 4;
        let is_envelope_direction_increase = is_bit(self.nr42, 3);
        let sweep_pace = self.nr42 & 0b111;

        let clock_shift = ((self.nr43 >> 4) & 0xF) as u32;
        let lfsr_short_mode = is_bit(self.nr43, 3);
        let clock_divider_raw = self.nr43 & 0b111;

        let length_enable = is_bit(self.nr44, 6);

        let length = 64 - initial_length_timer;
        let volume = init_volume as f32 / 15.0;
        let active = init_volume > 0 || is_envelope_direction_increase;
        let envelope_sweep_length = (44_100 * sweep_pace as u32) / 64;
        let lfsr_freq = 262144.0
            / ((NOISE_CHANNEL_DIVISORS[clock_divider_raw as usize] as u32) * (1 << clock_shift))
                as f32;

        {
            let mut packet = self.ch4_packet.lock().unwrap();
            packet.active = active;
            packet.length = length;
            packet.length_enable = length_enable;
            packet.volume = volume;
            packet.is_envelope_dir_inc = is_envelope_direction_increase;
            packet.lfsr = 0;
            packet.lfsr_short_mode = lfsr_short_mode;
            packet.freq = lfsr_freq;
            packet.envelope_sweep_length = envelope_sweep_length;
            packet.speaker_left = self.is_ch4_left();
            packet.speaker_right = self.is_ch4_right();
        }
    }

    fn is_ch4_left(&self) -> bool {
        is_bit(self.nr51, 7)
    }
    fn is_ch3_left(&self) -> bool {
        is_bit(self.nr51, 6)
    }
    fn is_ch2_left(&self) -> bool {
        is_bit(self.nr51, 5)
    }
    fn is_ch1_left(&self) -> bool {
        is_bit(self.nr51, 4)
    }
    fn is_ch4_right(&self) -> bool {
        is_bit(self.nr51, 3)
    }
    fn is_ch3_right(&self) -> bool {
        is_bit(self.nr51, 2)
    }
    fn is_ch2_right(&self) -> bool {
        is_bit(self.nr51, 1)
    }
    fn is_ch1_right(&self) -> bool {
        is_bit(self.nr51, 0)
    }
}
