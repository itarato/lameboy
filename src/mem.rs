use crate::cartridge::*;
use crate::conf::*;

pub struct Mem {
    pub boot_lock_reg: u8,
    pub bios: [u8; 0x100],
    hram: [u8; 0x7F],
    vram: Vram,
    oam_ram: OamVram,
    cartridge: Cartridge,
}

impl Mem {
    pub fn new(cartridge: Cartridge, vram: Vram, oam_ram: OamVram) -> Result<Self, Error> {
        Ok(Mem {
            boot_lock_reg: 0,
            bios: [0; 0x100],
            hram: [0; 0x7F],
            vram,
            oam_ram,
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
                self.cartridge.rom_0()[loc as usize]
            }
        } else if (MEM_AREA_VRAM_START..=MEM_AREA_VRAM_END).contains(&loc) {
            self.vram.lock().expect("Cannot lock vram")[(loc - MEM_AREA_VRAM_START) as usize]
        } else if (MEM_AREA_OAM_START..=MEM_AREA_OAM_END).contains(&loc) {
            self.oam_ram.lock().expect("Cannot lock oam ram")[(loc - MEM_AREA_OAM_START) as usize]
        } else if (MEM_AREA_HRAM_START..=MEM_AREA_HRAM_END).contains(&loc) {
            self.hram[(loc - MEM_AREA_HRAM_START) as usize]
        } else {
            return Err(format!("Unimplemented mem read: {:#06X}", loc).into());
        };

        log::debug!("Read: {:#06X} = #{:#04X}", loc, byte);

        Ok(byte)
    }

    pub fn write(&mut self, loc: u16, byte: u8) -> Result<(), Error> {
        if (MEM_AREA_VRAM_START..=MEM_AREA_VRAM_END).contains(&loc) {
            self.vram.lock().expect("Cannot lock vram")[(loc - MEM_AREA_VRAM_START) as usize] =
                byte;
        } else if (MEM_AREA_OAM_START..=MEM_AREA_OAM_END).contains(&loc) {
            self.oam_ram.lock().expect("Cannot lock oam ram")
                [(loc - MEM_AREA_OAM_START) as usize] = byte;
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
}
