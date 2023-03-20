use crate::cartridge::*;
use crate::conf::*;

pub struct Mem {
    pub bios: [u8; 0x100],
    cartridge: Cartridge,
    data: [u8; MEM_SIZE],
}

impl Mem {
    pub fn new(cartridge: Cartridge) -> Result<Self, Error> {
        Ok(Mem {
            bios: [0; 0x100],
            cartridge,
            data: [0u8; MEM_SIZE],
        })
    }

    pub fn reset(&mut self) -> Result<(), Error> {
        self.write_unchecked(MEM_LOC_BOOT_LOCK_REG, 0b0)?;
        Ok(())
    }

    pub fn read(&self, loc: u16) -> Result<u8, Error> {
        let byte = if loc <= MEM_AREA_ROM_BANK_0_END {
            if loc < BIOS_SIZE as u16 && self.is_bios_mounted() {
                self.bios[loc as usize]
            } else {
                unimplemented!("Read from MEM_AREA_ROM_BANK_0 not handled yet")
            }
        } else if loc <= MEM_AREA_ROM_BANK_N_END {
            unimplemented!("Read from MEM_AREA_ROM_BANK_N not handled yet")
        } else if loc <= MEM_AREA_VRAM_END {
            unimplemented!("Read from MEM_AREA_VRAM not handled yet")
        } else if loc <= MEM_AREA_EXTERNAL_END {
            unimplemented!("Read from MEM_AREA_EXTERNAL not handled yet")
        } else if loc <= MEM_AREA_WRAM_END {
            unimplemented!("Read from MEM_AREA_WRAM not handled yet")
        } else if loc <= MEM_AREA_WRAM_CGB_END {
            unimplemented!("Read from MEM_AREA_WRAM_CGB not handled yet")
        } else if loc <= MEM_AREA_ECHO_END {
            unimplemented!("Read from MEM_AREA_ECHO not handled yet")
        } else if loc <= MEM_AREA_OAM_END {
            unimplemented!("Read from MEM_AREA_OAM not handled yet")
        } else if loc <= MEM_AREA_PROHIBITED_END {
            unimplemented!("Read from MEM_AREA_PROHIBITED not handled yet")
        } else if loc <= MEM_AREA_IO_END {
            self.data[(loc - INNER_ROM_START_ADDR) as usize]
        } else if loc <= MEM_AREA_HRAM_END {
            unimplemented!("Read from MEM_AREA_HRAM not handled yet")
        } else if loc <= MEM_AREA_IE_END {
            unimplemented!("Read from MEM_AREA_IE not handled yet")
        } else {
            return Err("Read outside of memory".into());
        };

        log::debug!("Read: {:#06X} = #{:#04X}", loc, byte);

        Ok(byte)
    }

    pub fn read_unchecked(&self, loc: u16) -> Result<u8, Error> {
        if loc >= INNER_ROM_START_ADDR && loc <= MEM_ADDR_MAX {
            Ok(self.data[(loc - INNER_ROM_START_ADDR) as usize])
        } else {
            panic!("Unexpected unchecked read on: {:#06X}", loc)
        }
    }

    pub fn read_u16(&mut self, loc: u16) -> Result<u16, Error> {
        let lo = self.read(loc)?;
        let hi = self.read(loc + 1)?;
        Ok(((hi as u16) << 8) | lo as u16)
    }

    pub fn write(&mut self, loc: u16, byte: u8) -> Result<(), Error> {
        log::debug!("Write: {:#06X} = #{:#04X}", loc, byte);

        if loc <= MEM_AREA_ROM_BANK_0_END {
            return Err("Cannot write to ROM (0)".into());
        } else if loc <= MEM_AREA_ROM_BANK_N_END {
            return Err("Cannot write to ROM (N)".into());
        } else if loc <= MEM_AREA_VRAM_END {
            self.write_unchecked(loc, byte)
        } else if loc <= MEM_AREA_EXTERNAL_END {
            unimplemented!("Write to MEM_AREA_EXTERNAL is not implemented")
        } else if loc <= MEM_AREA_WRAM_END {
            unimplemented!("Write to MEM_AREA_WRAM is not implemented")
        } else if loc <= MEM_AREA_WRAM_CGB_END {
            unimplemented!("Write to MEM_AREA_WRAM_CGB is not implemented")
        } else if loc <= MEM_AREA_ECHO_END {
            unimplemented!("Write to MEM_AREA_ECHO is not implemented")
        } else if loc <= MEM_AREA_OAM_END {
            unimplemented!("Write to MEM_AREA_OAM is not implemented")
        } else if loc <= MEM_AREA_PROHIBITED_END {
            unimplemented!("Write to MEM_AREA_PROHIBITED is not implemented")
        } else if loc <= MEM_AREA_IO_END {
            match loc {
                MEM_LOC_P1 => unimplemented!("Write to register P1 is not implemented"),
                MEM_LOC_SB => unimplemented!("Write to register SB is not implemented"),
                MEM_LOC_SC => unimplemented!("Write to register SC is not implemented"),
                MEM_LOC_DIV => self.write_unchecked(MEM_LOC_DIV, 0),
                MEM_LOC_TIMA => unimplemented!("Write to register TIMA is not implemented"),
                MEM_LOC_TMA => unimplemented!("Write to register TMA is not implemented"),
                MEM_LOC_TAC => unimplemented!("Write to register TAC is not implemented"),
                MEM_LOC_IF => unimplemented!("Write to register IF is not implemented"),
                MEM_LOC_NR10 => unimplemented!("Write to register NR10 is not implemented"),
                MEM_LOC_NR11 => unimplemented!("Write to register NR11 is not implemented"),
                MEM_LOC_NR12 => unimplemented!("Write to register NR12 is not implemented"),
                MEM_LOC_NR13 => unimplemented!("Write to register NR13 is not implemented"),
                MEM_LOC_NR14 => unimplemented!("Write to register NR14 is not implemented"),
                MEM_LOC_NR21 => unimplemented!("Write to register NR21 is not implemented"),
                MEM_LOC_NR22 => unimplemented!("Write to register NR22 is not implemented"),
                MEM_LOC_NR23 => unimplemented!("Write to register NR23 is not implemented"),
                MEM_LOC_NR24 => unimplemented!("Write to register NR24 is not implemented"),
                MEM_LOC_NR30 => unimplemented!("Write to register NR30 is not implemented"),
                MEM_LOC_NR31 => unimplemented!("Write to register NR31 is not implemented"),
                MEM_LOC_NR32 => unimplemented!("Write to register NR32 is not implemented"),
                MEM_LOC_NR33 => unimplemented!("Write to register NR33 is not implemented"),
                MEM_LOC_NR34 => unimplemented!("Write to register NR34 is not implemented"),
                MEM_LOC_NR41 => unimplemented!("Write to register NR41 is not implemented"),
                MEM_LOC_NR42 => unimplemented!("Write to register NR42 is not implemented"),
                MEM_LOC_NR43 => unimplemented!("Write to register NR43 is not implemented"),
                MEM_LOC_NR44 => unimplemented!("Write to register NR44 is not implemented"),
                MEM_LOC_NR50 => unimplemented!("Write to register NR50 is not implemented"),
                MEM_LOC_NR51 => unimplemented!("Write to register NR51 is not implemented"),
                MEM_LOC_NR52 => unimplemented!("Write to register NR52 is not implemented"),
                MEM_LOC_LCDC => unimplemented!("Write to register LCDC is not implemented"),
                MEM_LOC_STAT => unimplemented!("Write to register STAT is not implemented"),
                MEM_LOC_SCY => unimplemented!("Write to register SCY is not implemented"),
                MEM_LOC_SCX => unimplemented!("Write to register SCX is not implemented"),
                MEM_LOC_LY => unimplemented!("Write to register LY is not implemented"),
                MEM_LOC_LYC => unimplemented!("Write to register LYC is not implemented"),
                MEM_LOC_DMA => unimplemented!("Write to register DMA is not implemented"),
                MEM_LOC_BGP => unimplemented!("Write to register BGP is not implemented"),
                MEM_LOC_OBP0 => unimplemented!("Write to register OBP0 is not implemented"),
                MEM_LOC_OBP1 => unimplemented!("Write to register OBP1 is not implemented"),
                MEM_LOC_WY => unimplemented!("Write to register WY is not implemented"),
                MEM_LOC_WX => unimplemented!("Write to register WX is not implemented"),
                MEM_LOC_KEY1 => unimplemented!("Write to register KEY1 is not implemented"),
                MEM_LOC_VBK => unimplemented!("Write to register VBK is not implemented"),
                MEM_LOC_BOOT_LOCK_REG => {
                    // BOOT_OFF can only transition from 0b0 to 0b1, so once 0b1 has been written, the boot ROM is
                    // permanently disabled until the next system reset. Writing 0b0 when BOOT_OFF is 0b0 has no
                    // effect and doesn’t lock the boot ROM.
                    if byte == 0b1 {
                        self.write_unchecked(loc, byte)
                    } else {
                        Err("Boot lock register must only be set to 1".into())
                    }
                }
                MEM_LOC_HDMA1 => unimplemented!("Write to register HDMA1 is not implemented"),
                MEM_LOC_HDMA2 => unimplemented!("Write to register HDMA2 is not implemented"),
                MEM_LOC_HDMA3 => unimplemented!("Write to register HDMA3 is not implemented"),
                MEM_LOC_HDMA4 => unimplemented!("Write to register HDMA4 is not implemented"),
                MEM_LOC_HDMA5 => unimplemented!("Write to register HDMA5 is not implemented"),
                MEM_LOC_RP => unimplemented!("Write to register RP is not implemented"),
                MEM_LOC_BCPS => unimplemented!("Write to register BCPS is not implemented"),
                MEM_LOC_BCPD => unimplemented!("Write to register BCPD is not implemented"),
                MEM_LOC_OCPS => unimplemented!("Write to register OCPS is not implemented"),
                MEM_LOC_OCPD => unimplemented!("Write to register OCPD is not implemented"),
                MEM_LOC_SVBK => unimplemented!("Write to register SVBK is not implemented"),
                _ => unimplemented!("Write to MEM_AREA_IO is not implemented"),
            }
        } else if loc <= MEM_AREA_HRAM_END {
            unimplemented!("Write to MEM_AREA_HRAM is not implemented")
        } else if loc <= MEM_AREA_IE_END {
            unimplemented!("Write to MEM_AREA_IE is not implemented")
        } else {
            return Err("Write outside of memory".into());
        }
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

    pub fn write_u16(&mut self, loc: u16, word: u16) -> Result<(), Error> {
        let hi = (word >> 8) as u8;
        let lo = (word & 0xFF) as u8;

        self.write(loc, lo)?;
        self.write(loc + 1, hi)?;

        Ok(())
    }

    fn is_bios_mounted(&self) -> bool {
        self.read_unchecked(MEM_LOC_BOOT_LOCK_REG).unwrap() == 0b0
    }
}
