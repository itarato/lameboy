use std::fs::File;
use std::io::Read;

use crate::conf::*;
use crate::cpu::*;
use crate::mem::*;

pub struct VM {
    mem: Mem,
    cpu: Cpu,
}

impl VM {
    pub fn new() -> Result<Self, Error> {
        Ok(VM {
            mem: Mem::new()?,
            cpu: Cpu::new(),
        })
    }

    pub fn setup(&mut self) -> Result<(), Error> {
        let bios = &mut self.mem.bios;
        let mut bios_file = File::open("assets/dmg_boot.bin")?;

        let bios_read_len = bios_file.read(bios)?;
        assert_eq!(BIOS_SIZE, bios_read_len, "BIOS read size not match");

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Error> {
        self.reset();

        loop {
            self.exec_op()?;

            unimplemented!()
        }
    }

    fn reset(&mut self) {}

    fn exec_op(&mut self) -> Result<(), Error> {
        let op = self.read_op()?;

        if (op >> 6) == 0b01 {
            if (op & 0b111) == 0b110 {
                // LD r, (HL): Load register (indirect HL)
                // Load to the 8-bit register r, data from the absolute address specified by the 16-bit register HL.
                // Opcode 0b01xxx110/various
                // Length 1 byte
                // Duration 2 machine cycles
                if op == 0x7E {
                    // LD A,(HL) 7E 8
                    self.cpu
                        .set_a(self.mem.read_absolute(self.cpu.hl as usize)?);
                } else if op == 0x46 {
                    // LD B,(HL) 46 8
                    self.cpu
                        .set_b(self.mem.read_absolute(self.cpu.hl as usize)?);
                } else if op == 0x4E {
                    // LD C,(HL) 4E 8
                    self.cpu
                        .set_c(self.mem.read_absolute(self.cpu.hl as usize)?);
                } else if op == 0x56 {
                    // LD D,(HL) 56 8
                    self.cpu
                        .set_d(self.mem.read_absolute(self.cpu.hl as usize)?);
                } else if op == 0x5E {
                    // LD E,(HL) 5E 8
                    self.cpu
                        .set_e(self.mem.read_absolute(self.cpu.hl as usize)?);
                } else if op == 0x66 {
                    // LD H,(HL) 66 8
                    self.cpu
                        .set_h(self.mem.read_absolute(self.cpu.hl as usize)?);
                } else if op == 0x6E {
                    // LD L,(HL) 6E 8
                    self.cpu
                        .set_l(self.mem.read_absolute(self.cpu.hl as usize)?);
                } else {
                    unimplemented!("Unhandled opcode (LD r, (HL)): 0x{:2X}", op);
                }

                self.cpu.mcycle += 2;
            } else if (op & 0b111) == 0b110 {
                // LD (HL), r: Load from register (indirect HL)
                // Load to the absolute address specified by the 16-bit register HL, data from the 8-bit register r.
                // Opcode 0b01110xxx/various
                // Length 1 byte
                // Duration 2 machine cycles

                if op == 0x70 {
                    // LD (HL),B 70 8
                    self.mem
                        .write_absolute(self.cpu.hl as usize, self.cpu.get_b())?;
                } else if op == 0x71 {
                    // LD (HL),C 71 8
                    self.mem
                        .write_absolute(self.cpu.hl as usize, self.cpu.get_c())?;
                } else if op == 0x72 {
                    // LD (HL),D 72 8
                    self.mem
                        .write_absolute(self.cpu.hl as usize, self.cpu.get_d())?;
                } else if op == 0x73 {
                    // LD (HL),E 73 8
                    self.mem
                        .write_absolute(self.cpu.hl as usize, self.cpu.get_e())?;
                } else if op == 0x74 {
                    // LD (HL),H 74 8
                    self.mem
                        .write_absolute(self.cpu.hl as usize, self.cpu.get_h())?;
                } else if op == 0x75 {
                    // LD (HL),L 75 8
                    self.mem
                        .write_absolute(self.cpu.hl as usize, self.cpu.get_l())?;
                } else if op == 0x77 {
                    // LD (HL),A 77 8
                    self.mem
                        .write_absolute(self.cpu.hl as usize, self.cpu.get_a())?;
                } else {
                    unimplemented!("Unhandled opcode (LD (HL), r): 0x{:2X}", op);
                }

                self.cpu.mcycle += 2;
            } else {
                // LD r, r’: Load register (register)
                // Load to the 8-bit register r, data from the 8-bit register r’.
                // Opcode 0b01xxxyyy/various
                // Length 1 byte
                // Duration 1 machine cycle

                if op == 0x7F {
                    // LD A,A 7F 4
                    let byte = self.cpu.get_a();
                    self.cpu.set_a(byte);
                } else if op == 0x78 {
                    // LD A,B 78 4
                    let byte = self.cpu.get_b();
                    self.cpu.set_a(byte);
                } else if op == 0x79 {
                    // LD A,C 79 4
                    let byte = self.cpu.get_c();
                    self.cpu.set_a(byte);
                } else if op == 0x7A {
                    // LD A,D 7A 4
                    let byte = self.cpu.get_d();
                    self.cpu.set_a(byte);
                } else if op == 0x7B {
                    // LD A,E 7B 4
                    let byte = self.cpu.get_e();
                    self.cpu.set_a(byte);
                } else if op == 0x7C {
                    // LD A,H 7C 4
                    let byte = self.cpu.get_h();
                    self.cpu.set_a(byte);
                } else if op == 0x7D {
                    // LD A,L 7D 4
                    let byte = self.cpu.get_l();
                    self.cpu.set_a(byte);
                } else if op == 0x40 {
                    // LD B,B 40 4
                    let byte = self.cpu.get_b();
                    self.cpu.set_b(byte);
                } else if op == 0x41 {
                    // LD B,C 41 4
                    let byte = self.cpu.get_c();
                    self.cpu.set_b(byte);
                } else if op == 0x42 {
                    // LD B,D 42 4
                    let byte = self.cpu.get_d();
                    self.cpu.set_b(byte);
                } else if op == 0x43 {
                    // LD B,E 43 4
                    let byte = self.cpu.get_e();
                    self.cpu.set_b(byte);
                } else if op == 0x44 {
                    // LD B,H 44 4
                    let byte = self.cpu.get_h();
                    self.cpu.set_b(byte);
                } else if op == 0x45 {
                    // LD B,L 45 4
                    let byte = self.cpu.get_l();
                    self.cpu.set_b(byte);
                } else if op == 0x48 {
                    // LD C,B 48 4
                    let byte = self.cpu.get_b();
                    self.cpu.set_c(byte);
                } else if op == 0x49 {
                    // LD C,C 49 4
                    let byte = self.cpu.get_c();
                    self.cpu.set_c(byte);
                } else if op == 0x4A {
                    // LD C,D 4A 4
                    let byte = self.cpu.get_d();
                    self.cpu.set_c(byte);
                } else if op == 0x4B {
                    // LD C,E 4B 4
                    let byte = self.cpu.get_e();
                    self.cpu.set_c(byte);
                } else if op == 0x4C {
                    // LD C,H 4C 4
                    let byte = self.cpu.get_h();
                    self.cpu.set_c(byte);
                } else if op == 0x4D {
                    // LD C,L 4D 4
                    let byte = self.cpu.get_l();
                    self.cpu.set_c(byte);
                } else if op == 0x50 {
                    // LD D,B 50 4
                    let byte = self.cpu.get_b();
                    self.cpu.set_d(byte);
                } else if op == 0x51 {
                    // LD D,C 51 4
                    let byte = self.cpu.get_c();
                    self.cpu.set_d(byte);
                } else if op == 0x52 {
                    // LD D,D 52 4
                    let byte = self.cpu.get_d();
                    self.cpu.set_d(byte);
                } else if op == 0x53 {
                    // LD D,E 53 4
                    let byte = self.cpu.get_e();
                    self.cpu.set_d(byte);
                } else if op == 0x54 {
                    // LD D,H 54 4
                    let byte = self.cpu.get_h();
                    self.cpu.set_d(byte);
                } else if op == 0x55 {
                    // LD D,L 55 4
                    let byte = self.cpu.get_l();
                    self.cpu.set_d(byte);
                } else if op == 0x58 {
                    // LD E,B 58 4
                    let byte = self.cpu.get_b();
                    self.cpu.set_e(byte);
                } else if op == 0x59 {
                    // LD E,C 59 4
                    let byte = self.cpu.get_c();
                    self.cpu.set_e(byte);
                } else if op == 0x5A {
                    // LD E,D 5A 4
                    let byte = self.cpu.get_d();
                    self.cpu.set_e(byte);
                } else if op == 0x5B {
                    // LD E,E 5B 4
                    let byte = self.cpu.get_e();
                    self.cpu.set_e(byte);
                } else if op == 0x5C {
                    // LD E,H 5C 4
                    let byte = self.cpu.get_h();
                    self.cpu.set_e(byte);
                } else if op == 0x5D {
                    // LD E,L 5D 4
                    let byte = self.cpu.get_l();
                    self.cpu.set_e(byte);
                } else if op == 0x60 {
                    // LD H,B 60 4
                    let byte = self.cpu.get_b();
                    self.cpu.set_h(byte);
                } else if op == 0x61 {
                    // LD H,C 61 4
                    let byte = self.cpu.get_c();
                    self.cpu.set_h(byte);
                } else if op == 0x62 {
                    // LD H,D 62 4
                    let byte = self.cpu.get_d();
                    self.cpu.set_h(byte);
                } else if op == 0x63 {
                    // LD H,E 63 4
                    let byte = self.cpu.get_e();
                    self.cpu.set_h(byte);
                } else if op == 0x64 {
                    // LD H,H 64 4
                    let byte = self.cpu.get_h();
                    self.cpu.set_h(byte);
                } else if op == 0x65 {
                    // LD H,L 65 4
                    let byte = self.cpu.get_l();
                    self.cpu.set_h(byte);
                } else if op == 0x68 {
                    // LD L,B 68 4
                    let byte = self.cpu.get_b();
                    self.cpu.set_l(byte);
                } else if op == 0x69 {
                    // LD L,C 69 4
                    let byte = self.cpu.get_c();
                    self.cpu.set_l(byte);
                } else if op == 0x6A {
                    // LD L,D 6A 4
                    let byte = self.cpu.get_d();
                    self.cpu.set_l(byte);
                } else if op == 0x6B {
                    // LD L,E 6B 4
                    let byte = self.cpu.get_e();
                    self.cpu.set_l(byte);
                } else if op == 0x6C {
                    // LD L,H 6C 4
                    let byte = self.cpu.get_h();
                    self.cpu.set_l(byte);
                } else if op == 0x6D {
                    // LD L,L 6D 4
                    let byte = self.cpu.get_l();
                    self.cpu.set_l(byte);
                } else {
                    unimplemented!("Unhandled opcode (LD r, r’): 0x{:2X}", op);
                }

                self.cpu.mcycle += 1;
            }
        } else if (op >> 6) == 0b00 && (op & 0b111) == 0b110 {
            // LD r, n: Load register (immediate)
            // Load to the 8-bit register r, the immediate data n.
            // Opcode 0b00xxx110/various + n
            // Length 2 byte
            // Duration 2 machine cycles
            let imm = self.read_op()?;

            if op == 0x06 {
                // LD B,n 06 8
                self.cpu.set_b(imm);
            } else if op == 0x0E {
                // LD C,n 0E 8
                self.cpu.set_c(imm);
            } else if op == 0x16 {
                // LD D,n 16 8
                self.cpu.set_d(imm);
            } else if op == 0x1E {
                // LD E,n 1E 8
                self.cpu.set_e(imm);
            } else if op == 0x26 {
                // LD H,n 26 8
                self.cpu.set_h(imm);
            } else if op == 0x2E {
                // LD L,n 2E 8
                self.cpu.set_l(imm);
            } else {
                unimplemented!("Unhandled opcode (LD r, n): 0x{:2X}", op);
            }

            self.cpu.mcycle += 2;
        } else {
            unimplemented!("Unhandled opcode: 0x{:2X}", op);
        }

        Ok(())
    }

    fn read_op(&mut self) -> Result<u8, Error> {
        let op = self.mem.read_absolute(self.cpu.pc as usize)?;
        self.cpu.pc = self.cpu.pc.wrapping_add(1);

        Ok(op)
    }
}
