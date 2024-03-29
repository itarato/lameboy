use crate::cartridge::*;
use crate::conf::*;

pub struct Mmu {
    pub boot_lock_reg: u8,
    pub bios: [u8; 0x100],
    hram: [u8; 0x7F],
    wram: [u8; WRAM_SIZE],
    cartridge: Cartridge,
}

impl Mmu {
    pub fn new(cartridge: Cartridge) -> Result<Self, Error> {
        Ok(Mmu {
            boot_lock_reg: 0,
            bios: [0; 0x100],
            hram: [0; 0x7F],
            wram: [0; WRAM_SIZE],
            cartridge,
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
                self.cartridge.read(loc)?
            }
        } else if (MEM_AREA_ROM_BANK_N_START..=MEM_AREA_ROM_BANK_N_END).contains(&loc) {
            self.cartridge.read(loc)?
        } else if (MEM_AREA_EXTERNAL_START..=MEM_AREA_EXTERNAL_END).contains(&loc) {
            self.cartridge.read(loc)?
        } else if (MEM_AREA_WRAM_START..=MEM_AREA_WRAM_END).contains(&loc) {
            self.wram[(loc - MEM_AREA_WRAM_START) as usize]
        } else if (MEM_AREA_HRAM_START..=MEM_AREA_HRAM_END).contains(&loc) {
            self.hram[(loc - MEM_AREA_HRAM_START) as usize]
        } else {
            return Err(format!("Unimplemented mem read: {:#06X}", loc).into());
        };

        log::debug!("Read: {:#06X} = #{:#04X}", loc, byte);

        Ok(byte)
    }

    pub fn write(&mut self, loc: u16, byte: u8) -> Result<(), Error> {
        if (0x0000..=0x7FFF).contains(&loc) {
            self.cartridge.write(loc, byte);
        } else if (MEM_AREA_EXTERNAL_START..=MEM_AREA_EXTERNAL_END).contains(&loc) {
            self.cartridge.write(loc, byte);
        } else if (MEM_AREA_WRAM_START..=MEM_AREA_WRAM_END).contains(&loc) {
            self.wram[(loc - MEM_AREA_WRAM_START) as usize] = byte;
        } else if (MEM_AREA_HRAM_START..=MEM_AREA_HRAM_END).contains(&loc) {
            self.hram[(loc - MEM_AREA_HRAM_START) as usize] = byte;
        } else {
            return Err(format!("Unimplemented mem write: {:#06X}", loc).into());
        }

        Ok(())
    }

    fn is_bios_mounted(&self) -> bool {
        self.boot_lock_reg == 0b0
    }

    pub fn rom_bank_selector(&self) -> u8 {
        self.cartridge.rom_bank_selector()
    }
}
