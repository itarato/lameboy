use std::{fs::File, io::Read};

use crate::conf::*;

pub struct Cartridge {
    data: Vec<u8>,
    mem_bank_n: usize,
    ram_enabled: bool,
}

impl Cartridge {
    pub fn new(filename: String) -> Result<Self, Error> {
        let mut data = vec![];

        let mut file = File::open(filename)?;
        file.read_to_end(&mut data)?;

        Ok(Cartridge {
            data,
            mem_bank_n: 1,
            ram_enabled: false,
        })
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

    pub fn write(&mut self, loc: u16, __byte: u8) {
        if (0x0000..=0x1FFF).contains(&loc) {
            // Before external RAM can be read or written, it must be enabled by writing $A to anywhere in
            // this address space. Any value with $A in the lower 4 bits enables the RAM attached to the MBC,
            // and any other value disables the RAM. It is unknown why $A is the value used to enable RAM.
            unimplemented!("Unimplemented write to RAM-ENABLE: {:#06X}", loc);

            // TODO: handle self.ram_enabled.
        } else if (MEM_AREA_EXTERNAL_START..=MEM_AREA_EXTERNAL_END).contains(&loc) {
            if self.ram_enabled {
                unimplemented!("External ram write is not implemented");
            } else {
                // This area is used to address external RAM in the cartridge (if any). The RAM is only accessible
                // if RAM is enabled, otherwise reads return open bus values (often $FF, but not guaranteed)
                // and writes are ignored.
            }
        } else {
            unimplemented!("Unimplemented write to cartridge: {:#06X}", loc);
        }
    }
}
