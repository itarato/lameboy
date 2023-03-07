use crate::conf::*;

pub struct Mem {
    data: [u8; MEM_SIZE],
}

impl Mem {
    pub fn new() -> Result<Self, Error> {
        Ok(Mem {
            data: [0u8; MEM_SIZE],
        })
    }

    pub fn write(&mut self, loc: usize, byte: u8) {}
}
