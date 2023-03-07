use crate::cartridge::*;
use crate::conf::*;

pub struct Mem {
    cartridge: Cartridge,
    data: [u8; MEM_SIZE],
}

impl Mem {
    pub fn new() -> Result<Self, Error> {
        Ok(Mem {
            cartridge: Cartridge::new()?,
            data: [0u8; MEM_SIZE],
        })
    }

    pub fn reset(&mut self) {
        self.force_write_absolute(MEM_LOC_BOOT_LOCK_REG, 0b0);
    }

    pub fn write_absolute(&mut self, loc: usize, byte: u8) -> Result<(), Error> {
        if loc < MEM_AREA_ROM_BANK_N {
            // MEM_AREA_ROM_BANK_0:
            unimplemented!()
        } else if loc < MEM_AREA_VRAM {
            // MEM_AREA_ROM_BANK_N:
            unimplemented!()
        } else if loc < MEM_AREA_EXTERNAL {
            // MEM_AREA_VRAM:
            unimplemented!()
        } else if loc < MEM_AREA_WRAM {
            // MEM_AREA_EXTERNAL:
            unimplemented!()
        } else if loc < MEM_AREA_WRAM_CGB {
            // MEM_AREA_WRAM:
            unimplemented!()
        } else if loc < MEM_AREA_ECHO {
            // MEM_AREA_WRAM_CGB:
            unimplemented!()
        } else if loc < MEM_AREA_OAM {
            // MEM_AREA_ECHO:
            unimplemented!()
        } else if loc < MEM_AREA_PROHIBITED {
            // MEM_AREA_OAM:
            unimplemented!()
        } else if loc < MEM_AREA_IO {
            // MEM_AREA_PROHIBITED:
            unimplemented!()
        } else if loc < MEM_AREA_HRAM {
            // MEM_AREA_IO:
            unimplemented!()
        } else if loc < MEM_AREA_IE {
            // MEM_AREA_HRAM:
            unimplemented!()
        } else if loc == MEM_AREA_IE {
            // MEM_AREA_IE:
            unimplemented!()
        } else {
            return Err("Write outside of memory".into());
        }

        Ok(())
    }

    fn force_write_absolute(&mut self, loc: usize, byte: u8) {
        assert!(loc >= MEM_AREA_VRAM, "Mem addr cannot write rom bank area");
        assert!(loc <= MEM_ADDR_MAX, "Mem addr cannot exceed limit");

        // Set pointer relative to non-rom-bank area.
        let loc = loc - MEM_AREA_VRAM;

        self.data[loc] = byte;
    }
}
