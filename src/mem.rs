use crate::cartridge::*;
use crate::conf::*;

pub struct Mem {
    pub bios: [u8; 0x100],
    cartridge: Cartridge,
    data: [u8; MEM_SIZE],
}

impl Mem {
    pub fn new() -> Result<Self, Error> {
        Ok(Mem {
            bios: [0; 0x100],
            cartridge: Cartridge::new()?,
            data: [0u8; MEM_SIZE],
        })
    }

    pub fn reset(&mut self) {
        self.write_unchecked(MEM_LOC_BOOT_LOCK_REG, 0b0);
    }

    pub fn read(&self, loc: u16) -> Result<u8, Error> {
        if loc < MEM_AREA_ROM_BANK_N {
            // MEM_AREA_ROM_BANK_0:
            if loc < BIOS_SIZE as u16 && self.is_bios_mounted() {
                Ok(self.bios[loc as usize])
            } else {
                unimplemented!("Read from MEM_AREA_ROM_BANK_0 not handled yet")
            }
        } else if loc < MEM_AREA_VRAM {
            // MEM_AREA_ROM_BANK_N:
            unimplemented!("Read from MEM_AREA_ROM_BANK_N not handled yet")
        } else if loc < MEM_AREA_EXTERNAL {
            // MEM_AREA_VRAM:
            unimplemented!("Read from MEM_AREA_VRAM not handled yet")
        } else if loc < MEM_AREA_WRAM {
            // MEM_AREA_EXTERNAL:
            unimplemented!("Read from MEM_AREA_EXTERNAL not handled yet")
        } else if loc < MEM_AREA_WRAM_CGB {
            // MEM_AREA_WRAM:
            unimplemented!("Read from MEM_AREA_WRAM not handled yet")
        } else if loc < MEM_AREA_ECHO {
            // MEM_AREA_WRAM_CGB:
            unimplemented!("Read from MEM_AREA_WRAM_CGB not handled yet")
        } else if loc < MEM_AREA_OAM {
            // MEM_AREA_ECHO:
            unimplemented!("Read from MEM_AREA_ECHO not handled yet")
        } else if loc < MEM_AREA_PROHIBITED {
            // MEM_AREA_OAM:
            unimplemented!("Read from MEM_AREA_OAM not handled yet")
        } else if loc < MEM_AREA_IO {
            // MEM_AREA_PROHIBITED:
            unimplemented!("Read from MEM_AREA_PROHIBITED not handled yet")
        } else if loc < MEM_AREA_HRAM {
            // MEM_AREA_IO:
            Ok(self.data[(loc - MEM_AREA_VRAM) as usize])
        } else if loc < MEM_AREA_IE {
            // MEM_AREA_HRAM:
            unimplemented!("Read from MEM_AREA_HRAM not handled yet")
        } else if loc == MEM_AREA_IE {
            // MEM_AREA_IE:
            unimplemented!("Read from MEM_AREA_IE not handled yet")
        } else {
            Err("Read outside of memory".into())
        }
    }

    pub fn read_u16(&mut self, loc: u16) -> Result<u16, Error> {
        let lo = self.read(loc)?;
        let hi = self.read(loc + 1)?;
        Ok(((hi as u16) << 8) | lo as u16)
    }

    pub fn write(&mut self, loc: u16, byte: u8) -> Result<(), Error> {
        match loc {
            MEM_LOC_BOOT_LOCK_REG => {
                // BOOT_OFF can only transition from 0b0 to 0b1, so once 0b1 has been written, the boot ROM is
                // permanently disabled until the next system reset. Writing 0b0 when BOOT_OFF is 0b0 has no
                // effect and doesn’t lock the boot ROM.
                assert_eq!(0b1, byte, "Boot lock register must only be set to 1");
            }
            _ => (),
        }

        if loc < MEM_AREA_ROM_BANK_N {
            // MEM_AREA_ROM_BANK_0:
            return Err("Cannot write to ROM (0)".into());
        } else if loc < MEM_AREA_VRAM {
            // MEM_AREA_ROM_BANK_N:
            return Err("Cannot write to ROM (N)".into());
        } else if loc < MEM_AREA_EXTERNAL {
            // MEM_AREA_VRAM:
            unimplemented!("Write to MEM_AREA_VRAM is not implemented")
        } else if loc < MEM_AREA_WRAM {
            // MEM_AREA_EXTERNAL:
            unimplemented!("Write to MEM_AREA_EXTERNAL is not implemented")
        } else if loc < MEM_AREA_WRAM_CGB {
            // MEM_AREA_WRAM:
            unimplemented!("Write to MEM_AREA_WRAM is not implemented")
        } else if loc < MEM_AREA_ECHO {
            // MEM_AREA_WRAM_CGB:
            unimplemented!("Write to MEM_AREA_WRAM_CGB is not implemented")
        } else if loc < MEM_AREA_OAM {
            // MEM_AREA_ECHO:
            unimplemented!("Write to MEM_AREA_ECHO is not implemented")
        } else if loc < MEM_AREA_PROHIBITED {
            // MEM_AREA_OAM:
            unimplemented!("Write to MEM_AREA_OAM is not implemented")
        } else if loc < MEM_AREA_IO {
            // MEM_AREA_PROHIBITED:
            unimplemented!("Write to MEM_AREA_PROHIBITED is not implemented")
        } else if loc < MEM_AREA_HRAM {
            // MEM_AREA_IO:
            unimplemented!("Write to MEM_AREA_IO is not implemented")
        } else if loc < MEM_AREA_IE {
            // MEM_AREA_HRAM:
            unimplemented!("Write to MEM_AREA_HRAM is not implemented")
        } else if loc == MEM_AREA_IE {
            // MEM_AREA_IE:
            unimplemented!("Write to MEM_AREA_IE is not implemented")
        } else {
            return Err("Write outside of memory".into());
        }
    }

    fn write_unchecked(&mut self, loc: u16, byte: u8) {
        assert!(loc >= MEM_AREA_VRAM, "Mem addr cannot write rom bank area");
        assert!(loc <= MEM_ADDR_MAX, "Mem addr cannot exceed limit");

        if loc == MEM_LOC_DMA {
            unimplemented!("DMA-CTRL write is not implemented yet.")
        }

        // Set pointer relative to non-rom-bank area.
        let loc = loc - MEM_AREA_VRAM;

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
        self.read(MEM_LOC_BOOT_LOCK_REG).unwrap() == 0b0
    }
}
