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

pub const MEM_LOC_P1: usize = 0xFF00;
pub const MEM_LOC_SB: usize = 0xFF01;
pub const MEM_LOC_SC: usize = 0xFF02;
pub const MEM_LOC_DIV: usize = 0xFF04;
pub const MEM_LOC_TIMA: usize = 0xFF05;
pub const MEM_LOC_TMA: usize = 0xFF06;
pub const MEM_LOC_TAC: usize = 0xFF07;
pub const MEM_LOC_IF: usize = 0xFF0F;
pub const MEM_LOC_NR10: usize = 0xFF10;
pub const MEM_LOC_NR11: usize = 0xFF11;
pub const MEM_LOC_NR12: usize = 0xFF12;
pub const MEM_LOC_NR13: usize = 0xFF13;
pub const MEM_LOC_NR14: usize = 0xFF14;
pub const MEM_LOC_NR21: usize = 0xFF16;
pub const MEM_LOC_NR22: usize = 0xFF17;
pub const MEM_LOC_NR23: usize = 0xFF18;
pub const MEM_LOC_NR24: usize = 0xFF19;
pub const MEM_LOC_NR30: usize = 0xFF1A;
pub const MEM_LOC_NR31: usize = 0xFF1B;
pub const MEM_LOC_NR32: usize = 0xFF1C;
pub const MEM_LOC_NR33: usize = 0xFF1D;
pub const MEM_LOC_NR34: usize = 0xFF1E;
pub const MEM_LOC_NR41: usize = 0xFF20;
pub const MEM_LOC_NR42: usize = 0xFF21;
pub const MEM_LOC_NR43: usize = 0xFF22;
pub const MEM_LOC_NR44: usize = 0xFF23;
pub const MEM_LOC_NR50: usize = 0xFF24;
pub const MEM_LOC_NR51: usize = 0xFF25;
pub const MEM_LOC_NR52: usize = 0xFF26;
pub const MEM_LOC_LCDC: usize = 0xFF40;
pub const MEM_LOC_STAT: usize = 0xFF41;
pub const MEM_LOC_SCY: usize = 0xFF42;
pub const MEM_LOC_SCX: usize = 0xFF43;
pub const MEM_LOC_LY: usize = 0xFF44;
pub const MEM_LOC_LYC: usize = 0xFF45;
pub const MEM_LOC_DMA: usize = 0xFF46;
pub const MEM_LOC_BGP: usize = 0xFF47;
pub const MEM_LOC_OBP0: usize = 0xFF48;
pub const MEM_LOC_OBP1: usize = 0xFF49;
pub const MEM_LOC_WY: usize = 0xFF4A;
pub const MEM_LOC_WX: usize = 0xFF4B;
pub const MEM_LOC_KEY1: usize = 0xFF4D;
pub const MEM_LOC_VBK: usize = 0xFF4F;
pub const MEM_LOC_BOOT_LOCK_REG: usize = 0xFF50;
pub const MEM_LOC_HDMA1: usize = 0xFF51;
pub const MEM_LOC_HDMA2: usize = 0xFF52;
pub const MEM_LOC_HDMA3: usize = 0xFF53;
pub const MEM_LOC_HDMA4: usize = 0xFF54;
pub const MEM_LOC_HDMA5: usize = 0xFF55;
pub const MEM_LOC_RP: usize = 0xFF56;
pub const MEM_LOC_BCPS: usize = 0xFF68;
pub const MEM_LOC_BCPD: usize = 0xFF69;
pub const MEM_LOC_OCPS: usize = 0xFF6A;
pub const MEM_LOC_OCPD: usize = 0xFF6B;
pub const MEM_LOC_SVBK: usize = 0xFF70;
pub const MEM_LOC_IE: usize = 0xFFFF;

pub const BIOS_SIZE: usize = 0x100;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
