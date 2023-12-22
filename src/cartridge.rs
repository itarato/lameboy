use std::{fs::File, io::Read};

use crate::conf::*;

pub struct Cartridge {
    data: Vec<u8>,
    mem_bank_n: usize,
}

impl Cartridge {
    pub fn new(filename: String) -> Result<Self, Error> {
        let mut data = vec![];

        let mut file = File::open(filename)?;
        file.read_to_end(&mut data)?;

        Ok(Cartridge {
            data,
            mem_bank_n: 1,
        })
    }

    pub fn rom_0(&self) -> &[u8] {
        self.rom_n(0)
    }

    pub fn rom_n(&self, n: usize) -> &[u8] {
        &self.data[(ROM_BANK_SIZE * n)..(ROM_BANK_SIZE * (n + 1))]
    }

    pub fn read(&self, loc: u16) -> Result<u8, Error> {
        let byte = if (MEM_AREA_ROM_BANK_0_START..=MEM_AREA_ROM_BANK_0_END).contains(&loc) {
            self.data[loc as usize]
        } else if (MEM_AREA_ROM_BANK_N_START..=MEM_AREA_ROM_BANK_N_END).contains(&loc) {
            assert!(self.mem_bank_n >= 1);
            let physical_loc = self.mem_bank_n * 0x4000 + (loc as usize - 0x4000);
            self.data[physical_loc]
        } else {
            return Err(format!("Unexpected catridge addr: {:#06X}", loc).into());
        };

        Ok(byte)
    }
}
