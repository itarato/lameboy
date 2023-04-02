use crate::cartridge::*;
use crate::conf::*;

pub struct Mem {
    pub boot_lock_reg: u8,
    pub bios: [u8; 0x100],
    cartridge: Cartridge,
    data: [u8; MEM_SIZE],
}

impl Mem {
    pub fn new(cartridge: Cartridge) -> Result<Self, Error> {
        Ok(Mem {
            boot_lock_reg: 0,
            bios: [0; 0x100],
            cartridge,
            data: [0u8; MEM_SIZE],
        })
    }

    pub fn reset(&mut self) -> Result<(), Error> {
        self.boot_lock_reg = 0;
        Ok(())
    }

    pub fn read(&self, loc: u16) -> Result<u8, Error> {
        let byte = if (MEM_AREA_ROM_BANK_0_START..=MEM_AREA_ROM_BANK_0_END).contains(&loc) {
            if loc < BIOS_SIZE as u16 && self.is_bios_mounted() {
                self.bios[loc as usize]
            } else {
                unimplemented!("Read from MEM_AREA_ROM_BANK_0 not handled yet")
            }
        } else if (MEM_AREA_ROM_BANK_N_START..=MEM_AREA_ROM_BANK_N_END).contains(&loc) {
            unimplemented!("Read from MEM_AREA_ROM_BANK_N not handled yet")
        } else if (MEM_AREA_VRAM_START..=MEM_AREA_OAM_END).contains(&loc) {
            self.data[(loc - INNER_ROM_START_ADDR) as usize]
        } else if (MEM_AREA_PROHIBITED_START..=MEM_AREA_IO_END).contains(&loc) {
            return Err(format!("Illegal mem read address: {:#06X}", loc).into());
        } else if (MEM_AREA_HRAM_START..=MEM_AREA_HRAM_END).contains(&loc) {
            self.data[(loc - INNER_ROM_START_ADDR) as usize]
        } else {
            return Err("Read outside of memory".into());
        };

        log::debug!("Read: {:#06X} = #{:#04X}", loc, byte);

        Ok(byte)
    }

    pub fn write_unchecked(&mut self, loc: u16, byte: u8) -> Result<(), Error> {
        if loc < INNER_ROM_START_ADDR {
            Err("Mem addr cannot write rom bank area".into())
        } else if loc > MEM_ADDR_MAX {
            Err("Mem addr cannot exceed limit".into())
        } else {
            // Set pointer relative to non-rom-bank area.
            let loc = loc - INNER_ROM_START_ADDR;

            self.data[loc as usize] = byte;

            Ok(())
        }
    }

    fn is_bios_mounted(&self) -> bool {
        self.boot_lock_reg == 0b0
    }
}
