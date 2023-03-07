use std::ops::Range;

// Half of the space, not including 2 x 16K rom bank mapping.
pub const MEM_SIZE: usize = 0x8000;

pub const MEM_ADDR_MAX: usize = 0xFFFF;

// 16 KiB ROM bank 00	From cartridge, usually a fixed bank
pub const MEM_AREA_ROM_BANK_0: usize = 0x0000;
// 16 KiB ROM Bank 01~NN	From cartridge, switchable bank via mapper (if any)
pub const MEM_AREA_ROM_BANK_N: usize = 0x4000;
// 8 KiB Video RAM (VRAM)	In CGB mode, switchable bank 0/1
pub const MEM_AREA_VRAM: usize = 0x8000;
// 8 KiB External RAM	From cartridge, switchable bank if any
pub const MEM_AREA_EXTERNAL: usize = 0xA000;
// 4 KiB Work RAM (WRAM)
pub const MEM_AREA_WRAM: usize = 0xC000;
// 4 KiB Work RAM (WRAM)	In CGB mode, switchable bank 1~7
pub const MEM_AREA_WRAM_CGB: usize = 0xD000;
// Mirror of C000~DDFF (ECHO RAM)	Nintendo says use of this area is prohibited.
pub const MEM_AREA_ECHO: usize = 0xE000;
// Sprite attribute table (OAM)
pub const MEM_AREA_OAM: usize = 0xFE00;
// Not Usable	Nintendo says use of this area is prohibited
pub const MEM_AREA_PROHIBITED: usize = 0xFEA0;
// I/O Registers
pub const MEM_AREA_IO: usize = 0xFF00;
// High RAM (HRAM)
pub const MEM_AREA_HRAM: usize = 0xFF80;
// Interrupt Enable register (IE)
pub const MEM_AREA_IE: usize = 0xFFFF;

pub const MEM_LOC_BOOT_LOCK_REG: usize = 0xFF50;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
