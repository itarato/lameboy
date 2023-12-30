use std::thread;
use std::time::Duration;

use rodio::dynamic_mixer;
use rodio::source::SineWave;
use rodio::OutputStream;
use rodio::Sink;
use rodio::Source;

use crate::conf::*;
use crate::util::*;

pub struct Sound {
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
}

impl Sound {
    pub fn new() -> Self {
        Sound {
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
        }
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
            // FF25 — NR51: Sound panning
            MEM_LOC_NR51 => self.nr51 = byte,
            // FF26 — NR52: Audio master control
            MEM_LOC_NR52 => {
                // Cannot manually set CHx enable/disable flags.
                assert!(byte & 0b1111 == 0);
                self.nr52 = byte;
            }
            _ => unimplemented!("Sound chip loc write: {:#06X} not implemented", loc),
        };
    }

    pub fn read(&self, loc: u16) -> Result<u8, Error> {
        unimplemented!("Sound chip read not implemented")
    }

    fn audio_on(&self) -> bool {
        is_bit(self.nr52, 7)
    }

    fn ch4_on(&self) -> bool {
        is_bit(self.nr52, 3)
    }

    fn ch3_on(&self) -> bool {
        is_bit(self.nr52, 2)
    }

    fn ch2_on(&self) -> bool {
        is_bit(self.nr52, 1)
    }

    fn ch1_on(&self) -> bool {
        is_bit(self.nr52, 0)
    }

    fn channel1_update(&self) {
        // Triggers channel.
        if !is_bit(self.nr14, 7) {
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
        let out_volume = (1.0 / 15.0) * init_volume as f32;

        println!("wave_duty={}", wave_duty);
        println!("init_length_timer={}", init_length_timer);
        println!("init_volume={}", init_volume);
        println!(
            "is_envelope_direction_increase={}",
            is_envelope_direction_increase
        );
        println!("sweep_pace={}", sweep_pace);
        println!("length_enable={}", length_enable);
        println!("period={}", period);

        thread::spawn(move || {
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let sink = Sink::try_new(&stream_handle).unwrap();
            let (controller, mixer) = dynamic_mixer::mixer::<f32>(1, 44_100);

            sink.append(mixer);

            let source = SineWave::new(out_freq)
                .take_duration(Duration::from_micros(100_000))
                .amplify(out_volume);

            controller.add(source);

            sink.sleep_until_end();
        });
    }

    fn channel2_update(&self) {
        log::error!("Channel 2 is not implemented");
    }
}
