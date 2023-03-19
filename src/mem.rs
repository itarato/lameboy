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

    pub fn reset(&mut self) {
        self.write_unchecked(MEM_LOC_BOOT_LOCK_REG, 0b0);
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
            // MEM_AREA_IE:
            unimplemented!("Read from MEM_AREA_IE not handled yet")
        } else {
            return Err("Read outside of memory".into());
        };

        log::debug!("Read: {:#06X} = #{:#04X}", loc, byte);

        Ok(byte)
    }

    fn read_unchecked(&self, loc: u16) -> Result<u8, Error> {
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

        match loc {
            MEM_LOC_BOOT_LOCK_REG => {
                // BOOT_OFF can only transition from 0b0 to 0b1, so once 0b1 has been written, the boot ROM is
                // permanently disabled until the next system reset. Writing 0b0 when BOOT_OFF is 0b0 has no
                // effect and doesn’t lock the boot ROM.
                assert_eq!(0b1, byte, "Boot lock register must only be set to 1");
            }
            _ => (),
        }

        if loc <= MEM_AREA_ROM_BANK_0_END {
            return Err("Cannot write to ROM (0)".into());
        } else if loc <= MEM_AREA_ROM_BANK_N_END {
            return Err("Cannot write to ROM (N)".into());
        } else if loc <= MEM_AREA_VRAM_END {
            Ok(self.write_unchecked(loc, byte))
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
            unimplemented!("Write to MEM_AREA_IO is not implemented")
        } else if loc <= MEM_AREA_HRAM_END {
            unimplemented!("Write to MEM_AREA_HRAM is not implemented")
        } else if loc <= MEM_AREA_IE_END {
            unimplemented!("Write to MEM_AREA_IE is not implemented")
        } else {
            return Err("Write outside of memory".into());
        }
    }

    fn write_unchecked(&mut self, loc: u16, byte: u8) {
        assert!(
            loc >= INNER_ROM_START_ADDR,
            "Mem addr cannot write rom bank area"
        );
        assert!(loc <= MEM_ADDR_MAX, "Mem addr cannot exceed limit");

        if loc == MEM_LOC_DMA {
            unimplemented!("DMA-CTRL write is not implemented yet.")
        }

        // Set pointer relative to non-rom-bank area.
        let loc = loc - INNER_ROM_START_ADDR;

        self.data[loc as usize] = byte;
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
