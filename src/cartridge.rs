use std::{fs::File, io::Read};

use crate::conf::*;

trait CartridgeController {
    fn set_register(&mut self, loc: u16, byte: u8);
    fn translate_addr(&self, virtual_loc: u16) -> PhysicalAddr;
}

enum RamGate {
    // All other values disable access to cartridge RAM
    DisableRamAccess,
    // 0b1010= enable access to cartridge RAM
    EnableRamAccess,
}

enum PhysicalAddr {
    Ok(u32),
    NotAccessible,
}

enum Bank2Mode {
    // 0b0= BANK2 affects only accesses to 0x4000-0x7FFF
    Mode0,
    // 0b1= BANK2 affects accesses to 0x0000-0x3FFF, 0x4000-0x7FFF, 0xA000-0xBFFF
    Mode1,
}

struct RomOnly;

impl CartridgeController for RomOnly {
    fn set_register(&mut self, loc: u16, byte: u8) {
        log::error!("No write for RomOnly regs: ({:#06X}) = {:#04X}", loc, byte);
    }

    fn translate_addr(&self, virtual_loc: u16) -> PhysicalAddr {
        if (MEM_AREA_ROM_BANK_0_START..=MEM_AREA_ROM_BANK_N_END).contains(&virtual_loc) {
            PhysicalAddr::Ok(virtual_loc as u32)
        } else {
            unimplemented!(
                "Unimplemented RomOnly read at virtual addr: {:#06X}",
                virtual_loc
            );
        }
    }
}

struct MBC1 {
    ram_gate_reg: RamGate,
    bank_1_reg: u8,
    bank_2_reg: u8,
    bank2_mode_reg: Bank2Mode,
    // Number of 16k (0x3fff) banks.
    rom_bank_size: usize,
    // Number of 8k (0x1fff) banks.
    ram_bank_size: usize,
}

impl MBC1 {
    fn new(rom_bank_size: usize, ram_bank_size: usize) -> MBC1 {
        MBC1 {
            ram_gate_reg: RamGate::DisableRamAccess,
            bank_1_reg: 1,
            bank_2_reg: 0,
            bank2_mode_reg: Bank2Mode::Mode0,
            rom_bank_size,
            ram_bank_size,
        }
    }
}

impl CartridgeController for MBC1 {
    fn set_register(&mut self, loc: u16, byte: u8) {
        if (0x0000..=0x1FFF).contains(&loc) {
            if byte & 0xF == 0b1010 {
                self.ram_gate_reg = RamGate::EnableRamAccess;
            } else {
                self.ram_gate_reg = RamGate::DisableRamAccess;
            }
        } else if (0x2000..=0x3FFF).contains(&loc) {
            let mut byte = byte & 0b0001_1111;
            if byte == 0 {
                byte = 1;
            }
            self.bank_1_reg = byte;
        } else if (0x4000..=0x5FFF).contains(&loc) {
            self.bank_2_reg = byte & 0b0011;
        } else if (0x6000..=0x7FFF).contains(&loc) {
            self.bank2_mode_reg = if byte & 1 == 1 {
                Bank2Mode::Mode1
            } else {
                Bank2Mode::Mode0
            };
        } else {
            unimplemented!("MBC1 reg update not implemented for addr {:#06X}", loc);
        }
    }

    fn translate_addr(&self, virtual_loc: u16) -> PhysicalAddr {
        if (MEM_AREA_ROM_BANK_0_START..=MEM_AREA_ROM_BANK_0_END).contains(&virtual_loc) {
            match self.bank2_mode_reg {
                Bank2Mode::Mode0 => PhysicalAddr::Ok(virtual_loc as u32),
                Bank2Mode::Mode1 => PhysicalAddr::Ok(
                    ((self.bank_2_reg as u32) << 19) as u32
                        | (virtual_loc as u32 & 0b11_1111_1111_1111),
                ),
            }
        } else if (MEM_AREA_ROM_BANK_N_START..=MEM_AREA_ROM_BANK_N_END).contains(&virtual_loc) {
            // To specify the upper two bits (bits 5-6) of the ROM Bank number (1 MiB ROM or larger carts only)
            let rom_bank_selector = if self.rom_bank_size >= 64 {
                ((self.bank_2_reg as u32) << 5) | self.bank_1_reg as u32
            } else {
                self.bank_1_reg as u32
            };

            let physical_addr =
                (virtual_loc & 0b11_1111_1111_1111) as u32 | (rom_bank_selector << 14);
            PhysicalAddr::Ok(physical_addr)
        } else if (MEM_AREA_EXTERNAL_START..=MEM_AREA_EXTERNAL_END).contains(&virtual_loc) {
            match self.ram_gate_reg {
                RamGate::EnableRamAccess => match self.bank2_mode_reg {
                    Bank2Mode::Mode0 => {
                        PhysicalAddr::Ok((virtual_loc - MEM_AREA_EXTERNAL_START) as u32)
                    }
                    Bank2Mode::Mode1 => {
                        if self.ram_bank_size >= 4 {
                            PhysicalAddr::Ok(
                                ((virtual_loc - MEM_AREA_EXTERNAL_START) as u32
                                    & 0b1_1111_1111_1111)
                                    | ((self.bank_2_reg as u32) << 13),
                            )
                        } else {
                            PhysicalAddr::Ok(
                                (virtual_loc - MEM_AREA_EXTERNAL_START) as u32 & 0b1_1111_1111_1111,
                            )
                        }
                    }
                },
                RamGate::DisableRamAccess => PhysicalAddr::NotAccessible,
            }
        } else {
            unimplemented!(
                "MBC1 addr translation not implemented: {:#06X}",
                virtual_loc
            );
        }
    }
}

pub struct Cartridge {
    data: Vec<u8>,
    ram: Vec<u8>,
    ctrl: Box<dyn CartridgeController + Send>,
}

impl Cartridge {
    pub fn new(filename: String) -> Result<Self, Error> {
        let mut data = vec![];

        let mut file = File::open(filename)?;
        file.read_to_end(&mut data)?;

        let mut ram_size = 0usize;

        let ctrl: Box<dyn CartridgeController + Send> = match data[0x0147] {
            0x00 => Box::new(RomOnly),
            0x01 | 0x02 | 0x03 => {
                let rom_bank_size_bit = data[0x0148];
                let rom_bank_size = if rom_bank_size_bit <= 8 {
                    2 << rom_bank_size_bit
                } else {
                    panic!("Large cartridges are not yet implemented");
                };

                let ram_bank_size_bit = data[0x0149];
                let ram_bank_size = match ram_bank_size_bit {
                    0x00 => 0,
                    0x02 => 1,
                    0x03 => 4,
                    0x04 => 16,
                    0x05 => 8,
                    _ => panic!("RAM bank size bit not implemented"),
                };
                ram_size = ram_bank_size * 0x2000;

                Box::new(MBC1::new(rom_bank_size, ram_bank_size))
            }
            code => unimplemented!("Unimplemented cartridge type: {}", code),
        };

        Ok(Cartridge {
            data,
            ctrl,
            ram: vec![0; ram_size],
        })
    }

    pub fn read(&self, loc: u16) -> Result<u8, Error> {
        let byte = if (MEM_AREA_ROM_BANK_0_START..=MEM_AREA_ROM_BANK_0_END).contains(&loc) {
            self.data[loc as usize]
        } else if (MEM_AREA_ROM_BANK_N_START..=MEM_AREA_ROM_BANK_N_END).contains(&loc) {
            match self.ctrl.translate_addr(loc) {
                PhysicalAddr::Ok(addr) => self.data[addr as usize],
                _ => return Err("Error when loading data from BANK N".into()),
            }
        } else if (MEM_AREA_EXTERNAL_START..=MEM_AREA_EXTERNAL_END).contains(&loc) {
            match self.ctrl.translate_addr(loc) {
                PhysicalAddr::Ok(addr) => self.ram[addr as usize],
                PhysicalAddr::NotAccessible => return Err("Error when reading from RAM".into()),
            }
        } else {
            return Err(format!("Unexpected catridge addr: {:#06X}", loc).into());
        };

        Ok(byte)
    }

    pub fn write(&mut self, loc: u16, byte: u8) {
        if (0x0000..=0x7FFF).contains(&loc) {
            self.ctrl.set_register(loc, byte);
        } else if (MEM_AREA_EXTERNAL_START..=MEM_AREA_EXTERNAL_END).contains(&loc) {
            match self.ctrl.translate_addr(loc) {
                PhysicalAddr::Ok(addr) => {
                    self.ram[addr as usize] = byte;
                }
                PhysicalAddr::NotAccessible => (),
            };
        } else {
            unimplemented!("Unimplemented write to cartridge: {:#06X}", loc);
        }
    }
}
