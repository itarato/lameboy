use std::sync::Arc;
use std::sync::Mutex;

use sdl2::audio::AudioCallback;
use sdl2::audio::AudioDevice;
use sdl2::audio::AudioSpecDesired;

use crate::conf::*;
use crate::util::*;

#[derive(Debug)]
struct PulseSoundPacket {
    is_on: bool,
    pitch: f32,                   // 1.0 .. ~k
    volume: f32,                  // 0.0 .. 1.0
    envelope_sweep_length: usize, // 22050 = 1s
    envelope_direction_down: bool,
    waveform: f32, // 0.0 .. 1.0
    length_enable: bool,
    length: u8,
    length_counter: Counter,
    speaker_left: bool,
    speaker_right: bool,
    global_volume_left: f32,
    global_volume_right: f32,
}

impl PulseSoundPacket {
    fn new() -> PulseSoundPacket {
        PulseSoundPacket {
            is_on: false,
            pitch: 0.0,
            volume: 0.0,
            envelope_sweep_length: 0,
            envelope_direction_down: true,
            waveform: 0.0,
            length_enable: false,
            length: 0,
            length_counter: Counter::new(CPU_HZ as u64 / 256),
            speaker_left: true,
            speaker_right: true,
            global_volume_left: 0.5,
            global_volume_right: 0.5,
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

#[derive(Debug)]
struct WaveSoundPacket {
    is_on: bool,
    length_counter: Counter,
    length: u8,
    volume: f32,
    length_enable: bool,
    wave_pattern: [u8; 16],
    tone_freq: f32,
    speaker_left: bool,
    speaker_right: bool,
    global_volume_left: f32,
    global_volume_right: f32,
}

impl WaveSoundPacket {
    fn new() -> WaveSoundPacket {
        WaveSoundPacket {
            is_on: false,
            length_counter: Counter::new(CPU_HZ as u64 / 256),
            length: 0,
            volume: 0.0,
            length_enable: false,
            wave_pattern: [0; 16],
            tone_freq: 0.0,
            speaker_left: true,
            speaker_right: true,
            global_volume_left: 0.5,
            global_volume_right: 0.5,
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

struct NoiseSoundPacket {
    is_on: bool,
    length_counter: Counter,
    length: u8,
    length_enable: bool,
    volume: f32,
    is_envelope_dir_inc: bool,
    envelop_sweep_length: u32,
    lfsr_width: u8,
    lfsr_freq: f32,
    speaker_left: bool,
    speaker_right: bool,
    global_volume_left: f32,
    global_volume_right: f32,
}

impl NoiseSoundPacket {
    fn new() -> NoiseSoundPacket {
        NoiseSoundPacket {
            is_on: false,
            length_counter: Counter::new(CPU_HZ as u64 / 256),
            length: 0,
            length_enable: false,
            volume: 0.0,
            is_envelope_dir_inc: false,
            envelop_sweep_length: 0,
            lfsr_width: 0,
            lfsr_freq: 0.0,
            speaker_left: true,
            speaker_right: true,
            global_volume_left: 0.5,
            global_volume_right: 0.5,
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

struct PulseChannel {
    freq: f32,
    phase: f32,
    packet: Arc<Mutex<PulseSoundPacket>>,
    envelope_sweep_counter: usize,
}

impl PulseChannel {
    fn generate(&mut self, out: &mut [f32], volume_divider: f32) {
        let mut packet = self.packet.lock().expect("Cannot lock pocket");

        if !packet.speaker_left && !packet.speaker_right {
            return;
        }

        for chunk in out.chunks_exact_mut(2) {
            let value = if !packet.is_on {
                0.0
            } else {
                if (*packet).envelope_sweep_length > 0 {
                    if self.envelope_sweep_counter > 0 {
                        self.envelope_sweep_counter -= 1;
                    } else {
                        (*packet).volume += if (*packet).envelope_direction_down {
                            -1f32 / 15f32
                        } else {
                            1f32 / 15f32
                        };
                        self.envelope_sweep_counter = (*packet).envelope_sweep_length;
                    }
                }

                if (*packet).volume < 0f32 {
                    (*packet).volume = 0.0;
                } else if (*packet).volume > 1f32 {
                    (*packet).volume = 1.0;
                }

                self.phase = (self.phase + (packet.pitch / self.freq)) % 1.0;
                if self.phase <= packet.waveform {
                    packet.volume
                } else {
                    -packet.volume
                }
            };

            // IDEA: Instead of fix dividing the volume by part-len, we could dynamically adjust so only sound made will decrease the rest.
            // Eg: adding the `idx`th sound to the sample: chunk[_] = (chunk[_] / idx) * (idx - 1) + value / idx
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

struct WaveChannel {
    freq: f32,
    phase: f32,
    packet: Arc<Mutex<WaveSoundPacket>>,
}

impl WaveChannel {
    fn generate(&mut self, out: &mut [f32], volume_divider: f32) {
        let packet = self.packet.lock().expect("Cannot lock pocket");

        if !packet.speaker_left && !packet.speaker_right {
            return;
        }

        for chunk in out.chunks_exact_mut(2) {
            let value = if !packet.is_on {
                0.0
            } else {
                self.phase = (self.phase + (packet.tone_freq / self.freq)) % 1.0;
                if self.phase <= 0.5 {
                    packet.volume
                } else {
                    -packet.volume
                }
            };

            // MISSING: applying waveform to the samples.

            // IDEA: Instead of fix dividing the volume by part-len, we could dynamically adjust so only sound made will decrease the rest.
            // Eg: adding the `idx`th sound to the sample: chunk[_] = (chunk[_] / idx) * (idx - 1) + value / idx
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

struct NoiseChannel {
    packet: Arc<Mutex<NoiseSoundPacket>>,
}

impl NoiseChannel {
    fn generate(&mut self, out: &mut [f32], volume_divider: f32) {}
}

struct DmgChannels {
    ch1_pulse: PulseChannel,
    ch2_pulse: PulseChannel,
    ch3_wave: WaveChannel,
    ch4_noise: NoiseChannel,
}

impl DmgChannels {
    fn new(
        freq: f32,
        ch1_packet: Arc<Mutex<PulseSoundPacket>>,
        ch2_packet: Arc<Mutex<PulseSoundPacket>>,
        ch3_packet: Arc<Mutex<WaveSoundPacket>>,
        ch4_packet: Arc<Mutex<NoiseSoundPacket>>,
    ) -> DmgChannels {
        DmgChannels {
            ch1_pulse: PulseChannel {
                freq,
                phase: 0.0,
                packet: ch1_packet,
                envelope_sweep_counter: 0,
            },
            ch2_pulse: PulseChannel {
                freq,
                phase: 0.0,
                packet: ch2_packet,
                envelope_sweep_counter: 0,
            },
            ch3_wave: WaveChannel {
                freq,
                phase: 0.0,
                packet: ch3_packet,
            },
            ch4_noise: NoiseChannel { packet: ch4_packet },
        }
    }
}

impl AudioCallback for DmgChannels {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // MUST BE EQUAL TO HOW MANY PARTS CONTRIBUTING TO THE DEVICE.
        const PARTS_LEN: f32 = 4.0;

        // Silence it out - so channels can _add_ their part.
        out.iter_mut().for_each(|b| *b = 0.0);

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

    _sound_device: AudioDevice<DmgChannels>,
    ch1_packet: Arc<Mutex<PulseSoundPacket>>,
    ch2_packet: Arc<Mutex<PulseSoundPacket>>,
    ch3_packet: Arc<Mutex<WaveSoundPacket>>,
    ch4_packet: Arc<Mutex<NoiseSoundPacket>>,
}

impl Apu {
    pub fn new(disable_sound: bool) -> Self {
        let sdl_context = sdl2::init().unwrap();

        let desired_spec = AudioSpecDesired {
            freq: Some(44_100),
            channels: Some(2),
            samples: None,
        };

        let ch1_packet = Arc::new(Mutex::new(PulseSoundPacket::new()));
        let ch2_packet = Arc::new(Mutex::new(PulseSoundPacket::new()));
        let ch3_packet = Arc::new(Mutex::new(WaveSoundPacket::new()));
        let ch4_packet = Arc::new(Mutex::new(NoiseSoundPacket::new()));

        let _sound_device = sdl_context
            .audio()
            .unwrap()
            .open_playback(None, &desired_spec, |spec| {
                DmgChannels::new(
                    spec.freq as _,
                    ch1_packet.clone(),
                    ch2_packet.clone(),
                    ch3_packet.clone(),
                    ch4_packet.clone(),
                )
            })
            .unwrap();
        if !disable_sound {
            _sound_device.resume();
        }

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
            _sound_device,
            ch1_packet,
            ch2_packet,
            ch3_packet,
            ch4_packet,
            disable_sound,
        }
    }

    pub fn update(&mut self, cycles: u64) {
        let mut disable_ch1 = false;
        let mut disable_ch2 = false;
        let mut disable_ch3 = false;
        let mut disable_ch4 = false;

        {
            let mut packet = self.ch1_packet.lock().unwrap();
            packet.tick(cycles);
            let disable_ch1 = !packet.is_on;
        }
        {
            let mut packet = self.ch2_packet.lock().unwrap();
            packet.tick(cycles);
            let disable_ch2 = !packet.is_on;
        }
        {
            let mut packet = self.ch3_packet.lock().unwrap();
            packet.tick(cycles);
            let disable_ch3 = !packet.is_on;
        }
        {
            let mut packet = self.ch4_packet.lock().unwrap();
            packet.tick(cycles);
            let disable_ch4 = !packet.is_on;
        }

        if disable_ch1 {
            self.ch1_disable();
        }
        if disable_ch2 {
            self.ch2_disable();
        }
        if disable_ch3 {
            self.ch3_disable();
        }
        if disable_ch4 {
            self.ch4_disable();
        }
    }

    pub fn write(&mut self, loc: u16, byte: u8) {
        match loc {
            // TODO: Maybe we need this:
            // "During the All Sound OFF mode, each sound mode register cannot be set.)"
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
            MEM_LOC_NR34 => {
                self.nr34 = byte;
                self.channel3_update();
            }

            MEM_LOC_NR41 => self.nr41 = byte,
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

                let speaker_left = is_bit(self.nr50, 7);
                let speaker_right = is_bit(self.nr50, 3);

                {
                    let mut packet = self.ch1_packet.lock().unwrap();
                    packet.global_volume_left = volume_left;
                    packet.global_volume_right = volume_right;
                }
                {
                    let mut packet = self.ch2_packet.lock().unwrap();
                    packet.global_volume_left = volume_left;
                    packet.global_volume_right = volume_right;
                }
                {
                    let mut packet = self.ch3_packet.lock().unwrap();
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
            MEM_LOC_NR51 => self.nr51 = byte,
            // FF26 — NR52: Audio master control
            MEM_LOC_NR52 => {
                // Cannot manually set CHx enable/disable flags.
                self.nr52 = byte & 0xF0;

                if !self.audio_on() {
                    // TODO: The packets needs to be updated too to have an instant off.
                    self.ch1_disable();
                    self.ch2_disable();
                    self.ch3_disable();
                    self.ch4_disable();
                }
            }
            MEM_LOC_WAVE_PATTERN_START..=MEM_LOC_WAVE_PATTERN_END => {
                if !self.is_ch3_on() {
                    self.wave_pattern_ram[(loc - MEM_LOC_WAVE_PATTERN_START) as usize] = byte;
                } else {
                    log::error!("Write to CH3 wave patterns while on");
                    // Make sure the turn-off mechanism works. If it is - error can be ignored.
                }
            }
            _ => unimplemented!("Apu chip loc write: {:#06X} not implemented", loc),
        };
    }

    pub fn read(&self, loc: u16) -> Result<u8, Error> {
        match loc {
            MEM_LOC_NR50 => Ok(self.nr50),
            MEM_LOC_NR51 => Ok(self.nr51),
            MEM_LOC_NR52 => Ok(self.nr52),
            MEM_LOC_WAVE_PATTERN_START..=MEM_LOC_WAVE_PATTERN_END => {
                if self.is_ch3_on() {
                    Ok(self.wave_pattern_ram[(loc - MEM_LOC_WAVE_PATTERN_START) as usize])
                } else {
                    Ok(0xFF)
                }
            }
            _ => Err(format!("Apu chip read not implemented: {:#06X}", loc).into()),
        }
    }

    fn audio_on(&self) -> bool {
        is_bit(self.nr52, 7)
    }

    fn ch1_enable(&mut self) {
        set_bit(self.nr52, 0, true);
    }
    fn ch2_enable(&mut self) {
        set_bit(self.nr52, 1, true);
    }
    fn ch3_enable(&mut self) {
        set_bit(self.nr52, 2, true);
    }
    fn ch4_enable(&mut self) {
        set_bit(self.nr52, 3, true);
    }
    fn ch1_disable(&mut self) {
        set_bit(self.nr52, 0, false);
    }
    fn ch2_disable(&mut self) {
        set_bit(self.nr52, 1, false);
    }
    fn ch3_disable(&mut self) {
        set_bit(self.nr52, 2, false);
    }
    fn ch4_disable(&mut self) {
        set_bit(self.nr52, 3, false);
    }

    fn is_ch3_on(&self) -> bool {
        is_bit(self.nr52, 2)
    }

    fn channel1_update(&mut self) {
        if self.disable_sound || !self.audio_on() || !is_bit(self.nr14, 7) {
            self.ch1_disable();
            return;
        }

        let pace = (self.nr10 >> 4) & 0b111;
        let direction = is_bit(self.nr10, 3);
        let individual_step = self.nr10 & 0b111;

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
        let is_on = init_volume > 0 || is_envelope_direction_increase;

        if is_on {
            self.ch1_enable();
        } else {
            self.ch1_disable();
        }

        {
            let mut packet = self.ch1_packet.lock().unwrap();
            packet.is_on = is_on;
            packet.pitch = out_freq;
            packet.volume = out_volume;
            packet.envelope_sweep_length = out_envelop_sweep_length;
            packet.envelope_direction_down = !is_envelope_direction_increase;
            packet.waveform = out_waveform;
            packet.length_enable = length_enable;
            packet.length = init_length_timer;
            packet.speaker_left = self.is_ch1_left();
            packet.speaker_right = self.is_ch1_right();
        }
    }

    fn channel2_update(&mut self) {
        if self.disable_sound || !self.audio_on() || !is_bit(self.nr24, 7) {
            self.ch2_disable();
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

        let is_on = init_volume > 0 || is_envelope_direction_increase;

        if is_on {
            self.ch2_enable();
        } else {
            self.ch2_disable();
        }

        {
            let mut packet = self.ch2_packet.lock().unwrap();
            packet.is_on = is_on;
            packet.pitch = out_freq;
            packet.volume = out_volume;
            packet.envelope_sweep_length = out_envelop_sweep_length;
            packet.envelope_direction_down = !is_envelope_direction_increase;
            packet.waveform = out_waveform;
            packet.length_enable = length_enable;
            packet.length = init_length_timer;
            packet.speaker_left = self.is_ch2_left();
            packet.speaker_right = self.is_ch2_right();
        }
    }

    fn channel3_update(&mut self) {
        if self.disable_sound || !self.audio_on() || !is_bit(self.nr34, 7) {
            self.ch3_disable();
            self.ch3_packet.lock().unwrap().is_on = false;
            // MISSING: Update length!
            return;
        }

        let dac_on = is_bit(self.nr30, 7);
        // The higher the length timer, the shorter the time before the channel is cut.
        let init_length_timer = self.nr31;
        assert!(init_length_timer <= 64);
        // 00	Mute (No sound)
        // 01	100% volume (use samples read from Wave RAM as-is)
        // 10	50% volume (shift samples read from Wave RAM right once)
        // 11	25% volume (shift samples read from Wave RAM right twice)
        let output_level = (self.nr32 >> 5) & 0b11;
        let period_lo = self.nr33 as u16;
        let period_hi = (self.nr34 & 0b111) as u16;
        let period = (period_hi << 8) | period_lo;
        let length_enable = is_bit(self.nr34, 6);
        let wave_pattern = self.wave_pattern_ram.clone();

        let tone_freq = (2097152.0 / (0x800 - period) as f32) / 32.0;
        let volume: f32 = match output_level {
            0 => 0.0,
            1 => 1.0,
            2 => 0.5,
            3 => 0.25,
            _ => unreachable!(),
        };
        let length = 64 - init_length_timer;
        let is_on = dac_on;

        if is_on {
            self.ch3_enable();
        } else {
            self.ch3_disable();
        }

        {
            let mut packet = self.ch3_packet.lock().unwrap();

            packet.is_on = is_on;
            packet.tone_freq = tone_freq;
            packet.length = length;
            packet.volume = volume;
            packet.length_enable = length_enable;
            packet.wave_pattern = wave_pattern;
            packet.speaker_left = self.is_ch3_left();
            packet.speaker_right = self.is_ch3_right();
        }
    }

    fn channel4_update(&mut self) {
        if self.disable_sound || !self.audio_on() || !is_bit(self.nr44, 7) {
            self.ch4_disable();
            return;
        }

        let initial_length_timer = self.nr41 & 0b11_1111;

        let init_volume = self.nr42 >> 4;
        let is_envelope_direction_increase = is_bit(self.nr42, 3);
        let sweep_pace = self.nr42 & 0b111;

        let clock_shift = ((self.nr43 >> 4) & 0xF) as u32;
        let lfsr_width_bit = is_bit(self.nr43, 3);
        let clock_divider_raw = self.nr43 & 0b111;

        let length_enable = is_bit(self.nr44, 6);

        let length = 64 - initial_length_timer;
        let volume = init_volume as f32 / 15.0;
        let is_on = init_volume > 0 || is_envelope_direction_increase;
        let envelop_sweep_length = (44_100 * sweep_pace as u32) / 64;
        let lfsr_width = if lfsr_width_bit { 7 } else { 15 };
        let clock_divider = if clock_divider_raw == 0 {
            0.5
        } else {
            clock_divider_raw as f32
        };
        let lfsr_freq = 262144.0 / (clock_divider * (1 << clock_shift) as f32);

        if is_on {
            self.ch4_enable();
        } else {
            self.ch4_disable();
        }

        {
            let mut packet = self.ch4_packet.lock().unwrap();
            packet.is_on = is_on;
            packet.length = length;
            packet.length_enable = length_enable;
            packet.volume = volume;
            packet.is_envelope_dir_inc = is_envelope_direction_increase;
            packet.lfsr_width = lfsr_width;
            packet.lfsr_freq = lfsr_freq;
            packet.envelop_sweep_length = envelop_sweep_length;
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
