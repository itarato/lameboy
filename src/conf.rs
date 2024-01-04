pub const DISPLAY_WIDTH: u32 = 160;
pub const DISPLAY_HEIGHT: u32 = 144;
pub const DISPLAY_PIXELS_COUNT: usize = (DISPLAY_WIDTH * DISPLAY_HEIGHT) as usize;

pub const FPS: u32 = 60;
pub const SECOND_IN_MICROSECOND: u32 = 1_000_000;
pub const ONE_FRAME_IN_MICROSECONDS: u32 = SECOND_IN_MICROSECOND / FPS;

pub const CYCLE_PER_MCYCLE: u8 = 4;

// Cycles per second.
pub const CPU_HZ: u32 = 4194304;

// Cycles per second.
const DIV_REG_UPDATE_HZ: u32 = 256;
/**
 * 1s = CPU_HZ cycle (4194304)
 * 1s = DIM 16384 update
 * 4194304 mcycle = 16384 update
 * 256 mcycle = 1 update
 */
pub const DIV_REG_UPDATE_PER_MCYCLE: u32 = DIV_REG_UPDATE_HZ;

pub const TIMA_UPDATE_PER_MCYCLE: [u32; 4] = [1024u32, 16u32, 64u32, 256u32];

/// 16 KiB ROM bank 00	From cartridge, usually a fixed bank.
pub const MEM_AREA_ROM_BANK_0_START: u16 = 0x0000;
pub const MEM_AREA_ROM_BANK_0_END: u16 = 0x3FFF;

/// 16 KiB ROM Bank 01~NN	From cartridge, switchable bank via mapper (if any).
pub const MEM_AREA_ROM_BANK_N_START: u16 = 0x4000;
pub const MEM_AREA_ROM_BANK_N_END: u16 = 0x7FFF;

/// 8 KiB PPU RAM (VRAM)	In CGB mode, switchable bank 0/1.
pub const MEM_AREA_VRAM_START: u16 = 0x8000;
pub const MEM_AREA_VRAM_END: u16 = 0x9FFF;

/// 8 KiB External RAM	From cartridge, switchable bank if any.
pub const MEM_AREA_EXTERNAL_START: u16 = 0xA000;
pub const MEM_AREA_EXTERNAL_END: u16 = 0xBFFF;

/// 4 KiB Work RAM (WRAM).
pub const MEM_AREA_WRAM_START: u16 = 0xC000;
pub const MEM_AREA_WRAM_END: u16 = 0xDFFF;

/// Mirror of C000~DDFF (ECHO RAM)	Nintendo says use of this area is prohibited.
// pub const MEM_AREA_ECHO_START: u16 = 0xE000;
pub const MEM_AREA_ECHO_END: u16 = 0xFDFF;

/// Sprite attribute table (OAM).
pub const MEM_AREA_OAM_START: u16 = 0xFE00;
pub const MEM_AREA_OAM_END: u16 = 0xFE9F;

/// Not Usable	Nintendo says use of this area is prohibited.
pub const MEM_AREA_PROHIBITED_START: u16 = 0xFEA0;
pub const MEM_AREA_PROHIBITED_END: u16 = 0xFEFF;

/// I/O Registers.
pub const MEM_AREA_IO_START: u16 = 0xFF00;
pub const MEM_AREA_IO_END: u16 = 0xFF7F;

/// High RAM (HRAM).
pub const MEM_AREA_HRAM_START: u16 = 0xFF80;
pub const MEM_AREA_HRAM_END: u16 = 0xFFFE;

/// Interrupt Enable register (IE).
// pub const MEM_AREA_IE_START: u16 = 0xFFFF;
// pub const MEM_AREA_IE_END: u16 = 0xFFFF;

pub const MEM_LOC_P1: u16 = 0xFF00;
pub const MEM_LOC_SB: u16 = 0xFF01;
pub const MEM_LOC_SC: u16 = 0xFF02;
pub const MEM_LOC_DIV: u16 = 0xFF04;
pub const MEM_LOC_TIMA: u16 = 0xFF05;
pub const MEM_LOC_TMA: u16 = 0xFF06;
pub const MEM_LOC_TAC: u16 = 0xFF07;
pub const MEM_LOC_IF: u16 = 0xFF0F;

pub const MEM_LOC_NR10: u16 = 0xFF10;
pub const MEM_LOC_NR11: u16 = 0xFF11;
pub const MEM_LOC_NR12: u16 = 0xFF12;
pub const MEM_LOC_NR13: u16 = 0xFF13;
pub const MEM_LOC_NR14: u16 = 0xFF14;
pub const MEM_LOC_NR21: u16 = 0xFF16;
pub const MEM_LOC_NR22: u16 = 0xFF17;
pub const MEM_LOC_NR23: u16 = 0xFF18;
pub const MEM_LOC_NR24: u16 = 0xFF19;
pub const MEM_LOC_NR30: u16 = 0xFF1A;
pub const MEM_LOC_NR31: u16 = 0xFF1B;
pub const MEM_LOC_NR32: u16 = 0xFF1C;
pub const MEM_LOC_NR33: u16 = 0xFF1D;
pub const MEM_LOC_NR34: u16 = 0xFF1E;
pub const MEM_LOC_NR41: u16 = 0xFF20;
pub const MEM_LOC_NR42: u16 = 0xFF21;
pub const MEM_LOC_NR43: u16 = 0xFF22;
pub const MEM_LOC_NR44: u16 = 0xFF23;
pub const MEM_LOC_NR50: u16 = 0xFF24;
pub const MEM_LOC_NR51: u16 = 0xFF25;
pub const MEM_LOC_NR52: u16 = 0xFF26;
pub const MEM_LOC_WAVE_PATTERN_START: u16 = 0xFF30;
pub const MEM_LOC_WAVE_PATTERN_END: u16 = 0xFF3F;

pub const MEM_LOC_LCDC: u16 = 0xFF40;
pub const MEM_LOC_STAT: u16 = 0xFF41;
pub const MEM_LOC_SCY: u16 = 0xFF42;
pub const MEM_LOC_SCX: u16 = 0xFF43;
pub const MEM_LOC_LY: u16 = 0xFF44;
pub const MEM_LOC_LYC: u16 = 0xFF45;
pub const MEM_LOC_DMA: u16 = 0xFF46;
pub const MEM_LOC_BGP: u16 = 0xFF47;
pub const MEM_LOC_OBP0: u16 = 0xFF48;
pub const MEM_LOC_OBP1: u16 = 0xFF49;
pub const MEM_LOC_WY: u16 = 0xFF4A;
pub const MEM_LOC_WX: u16 = 0xFF4B;
pub const MEM_LOC_KEY1: u16 = 0xFF4D;
pub const MEM_LOC_VBK: u16 = 0xFF4F;
pub const MEM_LOC_BOOT_LOCK_REG: u16 = 0xFF50;
pub const MEM_LOC_HDMA1: u16 = 0xFF51;
pub const MEM_LOC_HDMA2: u16 = 0xFF52;
pub const MEM_LOC_HDMA3: u16 = 0xFF53;
pub const MEM_LOC_HDMA4: u16 = 0xFF54;
pub const MEM_LOC_HDMA5: u16 = 0xFF55;
pub const MEM_LOC_RP: u16 = 0xFF56;
pub const MEM_LOC_BCPS: u16 = 0xFF68;
pub const MEM_LOC_BCPD: u16 = 0xFF69;
pub const MEM_LOC_OCPS: u16 = 0xFF6A;
pub const MEM_LOC_OCPD: u16 = 0xFF6B;
pub const MEM_LOC_SVBK: u16 = 0xFF70;

pub const MEM_LOC_IE: u16 = 0xFFFF;

pub const BIOS_SIZE: usize = 0x100;

pub const OPCODE_NAME: [&str; 256] = [
    "NOP 1 4",
    "LD BC,d16 3 12",
    "LD (BC),A 1 8",
    "INC BC 1 8",
    "INC B 1 4",
    "DEC B 1 4",
    "LD B,d8 2 8",
    "RLCA 1 4",
    "LD (a16),SP 3 20",
    "ADD HL,BC 1 8",
    "LD A,(BC) 1 8",
    "DEC BC 1 8",
    "INC C 1 4",
    "DEC C 1 4",
    "LD C,d8 2 8",
    "RRCA 1 4",
    "STOP 0 2 4",
    "LD DE,d16 3 12",
    "LD (DE),A 1 8",
    "INC DE 1 8",
    "INC D 1 4",
    "DEC D 1 4",
    "LD D,d8 2 8",
    "RLA 1 4",
    "JR r8 2 12",
    "ADD HL,DE 1 8",
    "LD A,(DE) 1 8",
    "DEC DE 1 8",
    "INC E 1 4",
    "DEC E 1 4",
    "LD E,d8 2 8",
    "RRA 1 4",
    "JR NZ,r8 2 12/8",
    "LD HL,d16 3 12",
    "LD (HL+),A 1 8",
    "INC HL 1 8",
    "INC H 1 4",
    "DEC H 1 4",
    "LD H,d8 2 8",
    "DAA 1 4",
    "JR Z,r8 2 12/8",
    "ADD HL,HL 1 8",
    "LD A,(HL+) 1 8",
    "DEC HL 1 8",
    "INC L 1 4",
    "DEC L 1 4",
    "LD L,d8 2 8",
    "CPL 1 4",
    "JR NC,r8 2 12/8",
    "LD SP,d16 3 12",
    "LD (HL-),A 1 8",
    "INC SP 1 8",
    "INC (HL) 1 12",
    "DEC (HL) 1 12",
    "LD (HL),d8 2 12",
    "SCF 1 4",
    "JR C,r8 2 12/8",
    "ADD HL,SP 1 8",
    "LD A,(HL-) 1 8",
    "DEC SP 1 8",
    "INC A 1 4",
    "DEC A 1 4",
    "LD A,d8 2 8",
    "CCF 1 4",
    "LD B,B 1 4",
    "LD B,C 1 4",
    "LD B,D 1 4",
    "LD B,E 1 4",
    "LD B,H 1 4",
    "LD B,L 1 4",
    "LD B,(HL) 1 8",
    "LD B,A 1 4",
    "LD C,B 1 4",
    "LD C,C 1 4",
    "LD C,D 1 4",
    "LD C,E 1 4",
    "LD C,H 1 4",
    "LD C,L 1 4",
    "LD C,(HL) 1 8",
    "LD C,A 1 4",
    "LD D,B 1 4",
    "LD D,C 1 4",
    "LD D,D 1 4",
    "LD D,E 1 4",
    "LD D,H 1 4",
    "LD D,L 1 4",
    "LD D,(HL) 1 8",
    "LD D,A 1 4",
    "LD E,B 1 4",
    "LD E,C 1 4",
    "LD E,D 1 4",
    "LD E,E 1 4",
    "LD E,H 1 4",
    "LD E,L 1 4",
    "LD E,(HL) 1 8",
    "LD E,A 1 4",
    "LD H,B 1 4",
    "LD H,C 1 4",
    "LD H,D 1 4",
    "LD H,E 1 4",
    "LD H,H 1 4",
    "LD H,L 1 4",
    "LD H,(HL) 1 8",
    "LD H,A 1 4",
    "LD L,B 1 4",
    "LD L,C 1 4",
    "LD L,D 1 4",
    "LD L,E 1 4",
    "LD L,H 1 4",
    "LD L,L 1 4",
    "LD L,(HL) 1 8",
    "LD L,A 1 4",
    "LD (HL),B 1 8",
    "LD (HL),C 1 8",
    "LD (HL),D 1 8",
    "LD (HL),E 1 8",
    "LD (HL),H 1 8",
    "LD (HL),L 1 8",
    "HALT 1 4",
    "LD (HL),A 1 8",
    "LD A,B 1 4",
    "LD A,C 1 4",
    "LD A,D 1 4",
    "LD A,E 1 4",
    "LD A,H 1 4",
    "LD A,L 1 4",
    "LD A,(HL) 1 8",
    "LD A,A 1 4",
    "ADD A,B 1 4",
    "ADD A,C 1 4",
    "ADD A,D 1 4",
    "ADD A,E 1 4",
    "ADD A,H 1 4",
    "ADD A,L 1 4",
    "ADD A,(HL) 1 8",
    "ADD A,A 1 4",
    "ADC A,B 1 4",
    "ADC A,C 1 4",
    "ADC A,D 1 4",
    "ADC A,E 1 4",
    "ADC A,H 1 4",
    "ADC A,L 1 4",
    "ADC A,(HL) 1 8",
    "ADC A,A 1 4",
    "SUB B 1 4",
    "SUB C 1 4",
    "SUB D 1 4",
    "SUB E 1 4",
    "SUB H 1 4",
    "SUB L 1 4",
    "SUB (HL) 1 8",
    "SUB A 1 4",
    "SBC A,B 1 4",
    "SBC A,C 1 4",
    "SBC A,D 1 4",
    "SBC A,E 1 4",
    "SBC A,H 1 4",
    "SBC A,L 1 4",
    "SBC A,(HL) 1 8",
    "SBC A,A 1 4",
    "AND B 1 4",
    "AND C 1 4",
    "AND D 1 4",
    "AND E 1 4",
    "AND H 1 4",
    "AND L 1 4",
    "AND (HL) 1 8",
    "AND A 1 4",
    "XOR B 1 4",
    "XOR C 1 4",
    "XOR D 1 4",
    "XOR E 1 4",
    "XOR H 1 4",
    "XOR L 1 4",
    "XOR (HL) 1 8",
    "XOR A 1 4",
    "OR B 1 4",
    "OR C 1 4",
    "OR D 1 4",
    "OR E 1 4",
    "OR H 1 4",
    "OR L 1 4",
    "OR (HL) 1 8",
    "OR A 1 4",
    "CP B 1 4",
    "CP C 1 4",
    "CP D 1 4",
    "CP E 1 4",
    "CP H 1 4",
    "CP L 1 4",
    "CP (HL) 1 8",
    "CP A 1 4",
    "RET NZ 1 20/8",
    "POP BC 1 12",
    "JP NZ,a16 3 16/12",
    "JP a16 3 16",
    "CALL NZ,a16 3 24/12",
    "PUSH BC 1 16",
    "ADD A,d8 2 8",
    "RST 00H 1 16",
    "RET Z 1 20/8",
    "RET 1 16",
    "JP Z,a16 3 16/12",
    "PREFIX CB 1 4",
    "CALL Z,a16 3 24/12",
    "CALL a16 3 24",
    "ADC A,d8 2 8",
    "RST 08H 1 16",
    "RET NC 1 20/8",
    "POP DE 1 12",
    "JP NC,a16 3 16/12",
    "Invalid",
    "CALL NC,a16 3 24/12",
    "PUSH DE 1 16",
    "SUB d8 2 8",
    "RST 10H 1 16",
    "RET C 1 20/8",
    "RETI 1 16",
    "JP C,a16 3 16/12",
    "Invalid",
    "CALL C,a16 3 24/12",
    "Invalid",
    "SBC A,d8 2 8",
    "RST 18H 1 16",
    "LDH (a8),A 2 12",
    "POP HL 1 12",
    "LD (C),A 2 8",
    "Invalid",
    "Invalid",
    "PUSH HL 1 16",
    "AND d8 2 8",
    "RST 20H 1 16",
    "ADD SP,r8 2 16",
    "JP (HL) 1 4",
    "LD (a16),A 3 16",
    "Invalid",
    "Invalid",
    "Invalid",
    "XOR d8 2 8",
    "RST 28H 1 16",
    "LDH A,(a8) 2 12",
    "POP AF 1 12",
    "LD A,(C) 2 8",
    "DI 1 4",
    "Invalid",
    "PUSH AF 1 16",
    "OR d8 2 8",
    "RST 30H 1 16",
    "LD HL,SP+r8 2 12",
    "LD SP,HL 1 8",
    "LD A,(a16) 3 16",
    "EI 1 4",
    "Invalid",
    "Invalid",
    "CP d8 2 8",
    "RST 38H 1 16",
];

pub const OPCODE_CB_NAME: [&str; 256] = [
    "RLC B 2 8F",
    "RLC C 2 8",
    "RLC D 2 8",
    "RLC E 2 8",
    "RLC H 2 8",
    "RLC L 2 8",
    "RLC (HL) 2 16",
    "RLC A 2 8",
    "RRC B 2 8",
    "RRC C 2 8",
    "RRC D 2 8",
    "RRC E 2 8",
    "RRC H 2 8",
    "RRC L 2 8",
    "RRC (HL) 2 16",
    "RRC A 2 8",
    "RL B 2 8",
    "RL C 2 8",
    "RL D 2 8",
    "RL E 2 8",
    "RL H 2 8",
    "RL L 2 8",
    "RL (HL) 2 16",
    "RL A 2 8",
    "RR B 2 8",
    "RR C 2 8",
    "RR D 2 8",
    "RR E 2 8",
    "RR H 2 8",
    "RR L 2 8",
    "RR (HL) 2 16",
    "RR A 2 8",
    "SLA B 2 8",
    "SLA C 2 8",
    "SLA D 2 8",
    "SLA E 2 8",
    "SLA H 2 8",
    "SLA L 2 8",
    "SLA (HL) 2 16",
    "SLA A 2 8",
    "SRA B 2 8",
    "SRA C 2 8",
    "SRA D 2 8",
    "SRA E 2 8",
    "SRA H 2 8",
    "SRA L 2 8",
    "SRA (HL) 2 16",
    "SRA A 2 8",
    "SWAP B 2 8",
    "SWAP C 2 8",
    "SWAP D 2 8",
    "SWAP E 2 8",
    "SWAP H 2 8",
    "SWAP L 2 8",
    "SWAP (HL) 2 16",
    "SWAP A 2 8",
    "SRL B 2 8",
    "SRL C 2 8",
    "SRL D 2 8",
    "SRL E 2 8",
    "SRL H 2 8",
    "SRL L 2 8",
    "SRL (HL) 2 16",
    "SRL A 2 8",
    "BIT 0,B 2 8",
    "BIT 0,C 2 8",
    "BIT 0,D 2 8",
    "BIT 0,E 2 8",
    "BIT 0,H 2 8",
    "BIT 0,L 2 8",
    "BIT 0,(HL) 2 16",
    "BIT 0,A 2 8",
    "BIT 1,B 2 8",
    "BIT 1,C 2 8",
    "BIT 1,D 2 8",
    "BIT 1,E 2 8",
    "BIT 1,H 2 8",
    "BIT 1,L 2 8",
    "BIT 1,(HL) 2 16",
    "BIT 1,A 2 8",
    "BIT 2,B 2 8",
    "BIT 2,C 2 8",
    "BIT 2,D 2 8",
    "BIT 2,E 2 8",
    "BIT 2,H 2 8",
    "BIT 2,L 2 8",
    "BIT 2,(HL) 2 16",
    "BIT 2,A 2 8",
    "BIT 3,B 2 8",
    "BIT 3,C 2 8",
    "BIT 3,D 2 8",
    "BIT 3,E 2 8",
    "BIT 3,H 2 8",
    "BIT 3,L 2 8",
    "BIT 3,(HL) 2 16",
    "BIT 3,A 2 8",
    "BIT 4,B 2 8",
    "BIT 4,C 2 8",
    "BIT 4,D 2 8",
    "BIT 4,E 2 8",
    "BIT 4,H 2 8",
    "BIT 4,L 2 8",
    "BIT 4,(HL) 2 16",
    "BIT 4,A 2 8",
    "BIT 5,B 2 8",
    "BIT 5,C 2 8",
    "BIT 5,D 2 8",
    "BIT 5,E 2 8",
    "BIT 5,H 2 8",
    "BIT 5,L 2 8",
    "BIT 5,(HL) 2 16",
    "BIT 5,A 2 8",
    "BIT 6,B 2 8",
    "BIT 6,C 2 8",
    "BIT 6,D 2 8",
    "BIT 6,E 2 8",
    "BIT 6,H 2 8",
    "BIT 6,L 2 8",
    "BIT 6,(HL) 2 16",
    "BIT 6,A 2 8",
    "BIT 7,B 2 8",
    "BIT 7,C 2 8",
    "BIT 7,D 2 8",
    "BIT 7,E 2 8",
    "BIT 7,H 2 8",
    "BIT 7,L 2 8",
    "BIT 7,(HL) 2 16",
    "BIT 7,A 2 8",
    "RES 0,B 2 8",
    "RES 0,C 2 8",
    "RES 0,D 2 8",
    "RES 0,E 2 8",
    "RES 0,H 2 8",
    "RES 0,L 2 8",
    "RES 0,(HL) 2 16",
    "RES 0,A 2 8",
    "RES 1,B 2 8",
    "RES 1,C 2 8",
    "RES 1,D 2 8",
    "RES 1,E 2 8",
    "RES 1,H 2 8",
    "RES 1,L 2 8",
    "RES 1,(HL) 2 16",
    "RES 1,A 2 8",
    "RES 2,B 2 8",
    "RES 2,C 2 8",
    "RES 2,D 2 8",
    "RES 2,E 2 8",
    "RES 2,H 2 8",
    "RES 2,L 2 8",
    "RES 2,(HL) 2 16",
    "RES 2,A 2 8",
    "RES 3,B 2 8",
    "RES 3,C 2 8",
    "RES 3,D 2 8",
    "RES 3,E 2 8",
    "RES 3,H 2 8",
    "RES 3,L 2 8",
    "RES 3,(HL) 2 16",
    "RES 3,A 2 8",
    "RES 4,B 2 8",
    "RES 4,C 2 8",
    "RES 4,D 2 8",
    "RES 4,E 2 8",
    "RES 4,H 2 8",
    "RES 4,L 2 8",
    "RES 4,(HL) 2 16",
    "RES 4,A 2 8",
    "RES 5,B 2 8",
    "RES 5,C 2 8",
    "RES 5,D 2 8",
    "RES 5,E 2 8",
    "RES 5,H 2 8",
    "RES 5,L 2 8",
    "RES 5,(HL) 2 16",
    "RES 5,A 2 8",
    "RES 6,B 2 8",
    "RES 6,C 2 8",
    "RES 6,D 2 8",
    "RES 6,E 2 8",
    "RES 6,H 2 8",
    "RES 6,L 2 8",
    "RES 6,(HL) 2 16",
    "RES 6,A 2 8",
    "RES 7,B 2 8",
    "RES 7,C 2 8",
    "RES 7,D 2 8",
    "RES 7,E 2 8",
    "RES 7,H 2 8",
    "RES 7,L 2 8",
    "RES 7,(HL) 2 16",
    "RES 7,A 2 8",
    "SET 0,B 2 8",
    "SET 0,C 2 8",
    "SET 0,D 2 8",
    "SET 0,E 2 8",
    "SET 0,H 2 8",
    "SET 0,L 2 8",
    "SET 0,(HL) 2 16",
    "SET 0,A 2 8",
    "SET 1,B 2 8",
    "SET 1,C 2 8",
    "SET 1,D 2 8",
    "SET 1,E 2 8",
    "SET 1,H 2 8",
    "SET 1,L 2 8",
    "SET 1,(HL) 2 16",
    "SET 1,A 2 8",
    "SET 2,B 2 8",
    "SET 2,C 2 8",
    "SET 2,D 2 8",
    "SET 2,E 2 8",
    "SET 2,H 2 8",
    "SET 2,L 2 8",
    "SET 2,(HL) 2 16",
    "SET 2,A 2 8",
    "SET 3,B 2 8",
    "SET 3,C 2 8",
    "SET 3,D 2 8",
    "SET 3,E 2 8",
    "SET 3,H 2 8",
    "SET 3,L 2 8",
    "SET 3,(HL) 2 16",
    "SET 3,A 2 8",
    "SET 4,B 2 8",
    "SET 4,C 2 8",
    "SET 4,D 2 8",
    "SET 4,E 2 8",
    "SET 4,H 2 8",
    "SET 4,L 2 8",
    "SET 4,(HL) 2 16",
    "SET 4,A 2 8",
    "SET 5,B 2 8",
    "SET 5,C 2 8",
    "SET 5,D 2 8",
    "SET 5,E 2 8",
    "SET 5,H 2 8",
    "SET 5,L 2 8",
    "SET 5,(HL) 2 16",
    "SET 5,A 2 8",
    "SET 6,B 2 8",
    "SET 6,C 2 8",
    "SET 6,D 2 8",
    "SET 6,E 2 8",
    "SET 6,H 2 8",
    "SET 6,L 2 8",
    "SET 6,(HL) 2 16",
    "SET 6,A 2 8",
    "SET 7,B 2 8",
    "SET 7,C 2 8",
    "SET 7,D 2 8",
    "SET 7,E 2 8",
    "SET 7,H 2 8",
    "SET 7,L 2 8",
    "SET 7,(HL) 2 16",
    "SET 7,A 2 8",
];

#[rustfmt::skip]
pub const OPCODE_MCYCLE: [u8; 256] = [
    1, 3, 2, 2, 1, 1, 2, 1, 5, 2, 2, 2, 1, 1, 2, 1,
    1, 3, 2, 2, 1, 1, 2, 1, 3, 2, 2, 2, 1, 1, 2, 1,
    3, 3, 2, 2, 1, 1, 2, 1, 3, 2, 2, 2, 1, 1, 2, 1,
    3, 3, 2, 2, 3, 3, 3, 1, 3, 2, 2, 2, 1, 1, 2, 1,
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1,
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1,
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1,
    2, 2, 2, 2, 2, 2, 1, 2, 1, 1, 1, 1, 1, 1, 2, 1,
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1,
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1,
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1,
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1,
    5, 3, 4, 4, 6, 4, 2, 4, 5, 4, 4, 1, 6, 6, 2, 4,
    5, 3, 4, 0, 6, 4, 2, 4, 5, 4, 4, 0, 6, 0, 2, 4,
    3, 3, 2, 0, 0, 4, 2, 4, 4, 1, 4, 0, 0, 0, 2, 4,
    3, 3, 2, 1, 0, 4, 2, 4, 3, 2, 4, 1, 0, 0, 2, 4,
];

#[rustfmt::skip]
pub const OPCODE_MCYCLE_ALT: [u8; 256] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    2, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0,
    2, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    2, 0, 3, 0, 3, 0, 0, 0, 2, 0, 3, 0, 3, 0, 0, 0,
    2, 0, 3, 0, 3, 0, 0, 0, 2, 0, 3, 0, 3, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

#[rustfmt::skip]
pub const OPCODE_MCYCLE_PREFIX: [u8; 256] = [
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2,
];

pub const VRAM_SIZE: usize = (MEM_AREA_OAM_END - MEM_AREA_VRAM_START + 1) as usize;
pub const WRAM_SIZE: usize = (MEM_AREA_WRAM_END - MEM_AREA_WRAM_START + 1) as usize;
pub const OAM_RAM_SIZE: usize = (MEM_AREA_OAM_END - MEM_AREA_OAM_START + 1) as usize;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
