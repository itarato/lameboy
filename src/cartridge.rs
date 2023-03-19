use std::{fs::File, io::Read};

use crate::conf::*;

pub struct Cartridge {
    data: Vec<u8>,
}

impl Cartridge {
    pub fn new(filename: String) -> Result<Self, Error> {
        let mut data = vec![];

        let mut file = File::open(filename)?;
        file.read_to_end(&mut data)?;

        Ok(Cartridge { data })
    }

    pub fn rom_0(&self) -> &[u8] {
        self.rom_n(0)
    }

    pub fn rom_n(&self, n: usize) -> &[u8] {
        &self.data[(ROM_BANK_SIZE * n)..(ROM_BANK_SIZE * (n + 1))]
    }
}
