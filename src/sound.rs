use crate::conf::*;
use crate::conf::*;
use crate::mem::*;
use crate::util::*;

pub struct Sound {
    is_on: bool,
}

impl Sound {
    pub fn new() -> Self {
        Sound { is_on: false }
    }

    pub fn write(&mut self, loc: u16, byte: u8) -> Result<(), Error> {
        unimplemented!("Sound chip not implemented")
    }

    pub fn read(&self, loc: u16) -> Result<u8, Error> {
        unimplemented!("Sound chip not implemented")
    }
}
