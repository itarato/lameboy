use std::ops::Range;

// Half of the space, not including 2 x 16K rom bank mapping.
pub const MEM_SIZE: usize = 0x8000;

pub const MEM_ADDR_MAX: usize = 0xFFFF;

// 16 KiB ROM bank 00	From cartridge, usually a fixed bank
pub const MEM_AREA_ROM_BANK_00: Range<usize> = 0x0000..0x3FFF;
// 16 KiB ROM Bank 01~NN	From cartridge, switchable bank via mapper (if any)
pub const MEM_AREA_ROM_BANK_N: Range<usize> = 0x4000..0x7FFF;
// 8 KiB Video RAM (VRAM)	In CGB mode, switchable bank 0/1
pub const MEM_AREA_VRAM: Range<usize> = 0x8000..0x9FFF;
// 8 KiB External RAM	From cartridge, switchable bank if any
pub const MEM_AREA_EXTERNAL: Range<usize> = 0xA000..0xBFFF;
// 4 KiB Work RAM (WRAM)
pub const MEM_AREA_WRAM: Range<usize> = 0xC000..0xCFFF;
// 4 KiB Work RAM (WRAM)	In CGB mode, switchable bank 1~7
pub const MEM_AREA_WRAM_CGB: Range<usize> = 0xD000..0xDFFF;
// Mirror of C000~DDFF (ECHO RAM)	Nintendo says use of this area is prohibited.
pub const MEM_AREA_ECHO: Range<usize> = 0xE000..0xFDFF;
// Sprite attribute table (OAM)
pub const MEM_AREA_OAM: Range<usize> = 0xFE00..0xFE9F;
// Not Usable	Nintendo says use of this area is prohibited
pub const MEM_AREA_PROHIBITED: Range<usize> = 0xFEA0..0xFEFF;
// I/O Registers
pub const MEM_AREA_IO: Range<usize> = 0xFF00..0xFF7F;
// High RAM (HRAM)
pub const MEM_AREA_HRAM: Range<usize> = 0xFF80..0xFFFE;
// Interrupt Enable register (IE)
pub const MEM_AREA_IE: Range<usize> = 0xFFFF..0xFFFF;

pub const MEM_LOC_BOOT_LOCK_REG: usize = 0xFF50;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
