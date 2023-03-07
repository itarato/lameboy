use crate::conf::*;

pub struct Mem<'a> {
    rom_bank_0: &'a [u8],
    rom_bank_n: &'a [u8],
    data: [u8; MEM_SIZE],
}

impl<'a> Mem<'a> {
    pub fn new(rom_bank_0: &'a [u8], rom_bank_n: &'a [u8]) -> Result<Self, Error> {
        Ok(Mem {
            rom_bank_0,
            rom_bank_n,
            data: [0u8; MEM_SIZE],
        })
    }

    pub fn reset(&mut self) {
        self.force_write_absolute(MEM_LOC_BOOT_LOCK_REG, 0b0);
    }

    pub fn write_absolute(&mut self, loc: usize, byte: u8) {
        unimplemented!()
    }

    fn force_write_absolute(&mut self, loc: usize, byte: u8) {
        assert!(
            loc > MEM_AREA_ROM_BANK_N.end,
            "Mem addr cannot write rom bank area"
        );
        assert!(loc <= MEM_ADDR_MAX, "Mem addr cannot exceed limit");

        // Set pointer relative to non-rom-bank area.
        let loc = loc - MEM_AREA_ROM_BANK_N.end - 1;

        self.data[loc] = byte;
    }
}
