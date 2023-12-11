use crate::conf::*;

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
            MEM_LOC_NR11 => self.nr11 = byte,
            // NR13: Channel 1 period low [write-only].
            MEM_LOC_NR13 => self.nr13 = byte,
            // FF14 — NR14: Channel 1 period high & control.
            MEM_LOC_NR14 => self.nr14 = byte,
            MEM_LOC_NR12 => self.nr12 = byte,
            MEM_LOC_NR50 => self.nr50 = byte,
            MEM_LOC_NR51 => self.nr51 = byte,
            MEM_LOC_NR52 => self.nr52 = byte,
            _ => unimplemented!("Sound chip loc write: {:#06X} not implemented", loc),
        };
    }

    pub fn read(&self, loc: u16) -> Result<u8, Error> {
        unimplemented!("Sound chip read not implemented")
    }
}
