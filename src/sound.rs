use crate::conf::*;
use crate::mem::*;

pub struct Sound {
    is_on: bool,
}

impl Sound {
    pub fn new() -> Self {
        Sound { is_on: false }
    }

    pub fn update(&mut self, mem: &Mem) {
        let nr52 = mem.read_unchecked(MEM_LOC_NR52);
    }
}
