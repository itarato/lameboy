use std::fs::File;
use std::io::Read;

use crate::conf::*;
use crate::cpu::*;
use crate::mem::*;
use crate::util::*;

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

        log::info!("VM setup");

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Error> {
        self.reset();

        log::info!("VM eval loop start");

        loop {
            self.exec_op()?;
        }
    }

    fn reset(&mut self) {
        self.mem.reset();

        log::info!("VM reset");
    }

    fn exec_op(&mut self) -> Result<(), Error> {
        let op = self.read_op()?;
        log::debug!(">> {} (0x{:2X?})", opcode_name[op as usize], op);

        match op {
            0x00 => {
                // NOP 1 4 | - - - -
                unimplemented!("Opcode 0x00 (NOP 1 4) not implemented");
            }
            0x01 => {
                // LD BC,d16 3 12 | - - - -
                let word = self.read_op_imm16()?;
                self.cpu.bc = word;
            }
            0x02 => {
                // LD (BC),A 1 8 | - - - -
                let byte = self.cpu.get_a();
                self.mem.write(self.cpu.bc, byte)?;
            }
            0x03 => {
                // INC BC 1 8 | - - - -
                self.cpu.bc = self.cpu.bc.wrapping_add(1);
            }
            0x04 => {
                // INC B 1 4 | Z 0 H -
                let is_half_carry = is_half_carry_add_u8(self.cpu.get_b(), 1);
                let byte = self.cpu.get_b().wrapping_add(1);

                self.cpu.set_b(byte);
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(false);
                self.cpu.set_fh(is_half_carry);
            }
            0x05 => {
                // DEC B 1 4 | Z 1 H -
                let is_half_carry = is_half_carry_sub_u8(self.cpu.get_b(), 1);
                let byte = self.cpu.get_b().wrapping_sub(1);

                self.cpu.set_b(byte);
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(true);
                self.cpu.set_fh(is_half_carry);
            }
            0x06 => {
                // LD B,d8 2 8 | - - - -
                let byte = self.read_op()?;
                self.cpu.set_b(byte);
            }
            0x07 => {
                // RLCA 1 4 | 0 0 0 C
                unimplemented!("Opcode 0x07 (RLCA 1 4) not implemented");
            }
            0x08 => {
                // LD (a16),SP 3 20 | - - - -
                let word = self.read_op_imm16()?;
                self.mem.write_u16(word, self.cpu.sp)?;
            }
            0x09 => {
                // ADD HL,BC 1 8 | - 0 H C
                unimplemented!("Opcode 0x09 (ADD HL,BC 1 8) not implemented");
            }
            0x0A => {
                // LD A,(BC) 1 8 | - - - -
                let byte = self.mem.read(self.cpu.bc)?;
                self.cpu.set_a(byte);
            }
            0x0B => {
                // DEC BC 1 8 | - - - -
                self.cpu.bc = self.cpu.bc.wrapping_sub(1);
            }
            0x0C => {
                // INC C 1 4 | Z 0 H -
                let is_half_carry = is_half_carry_add_u8(self.cpu.get_c(), 1);
                let byte = self.cpu.get_c().wrapping_add(1);

                self.cpu.set_c(byte);
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(false);
                self.cpu.set_fh(is_half_carry);
            }
            0x0D => {
                // DEC C 1 4 | Z 1 H -
                let is_half_carry = is_half_carry_sub_u8(self.cpu.get_c(), 1);
                let byte = self.cpu.get_c().wrapping_sub(1);

                self.cpu.set_c(byte);
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(true);
                self.cpu.set_fh(is_half_carry);
            }
            0x0E => {
                // LD C,d8 2 8 | - - - -
                let byte = self.read_op()?;
                self.cpu.set_c(byte);
            }
            0x0F => {
                // RRCA 1 4 | 0 0 0 C
                unimplemented!("Opcode 0x0F (RRCA 1 4) not implemented");
            }
            0x10 => {
                // STOP 0 2 4 | - - - -
                unimplemented!("Opcode 0x10 (STOP 0 2 4) not implemented");
            }
            0x11 => {
                // LD DE,d16 3 12 | - - - -
                let word = self.read_op_imm16()?;
                self.cpu.de = word;
            }
            0x12 => {
                // LD (DE),A 1 8 | - - - -
                let byte = self.cpu.get_a();
                self.mem.write(self.cpu.de, byte)?;
            }
            0x13 => {
                // INC DE 1 8 | - - - -
                self.cpu.de = self.cpu.de.wrapping_add(1);
            }
            0x14 => {
                // INC D 1 4 | Z 0 H -
                let is_half_carry = is_half_carry_add_u8(self.cpu.get_d(), 1);
                let byte = self.cpu.get_d().wrapping_add(1);

                self.cpu.set_d(byte);
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(false);
                self.cpu.set_fh(is_half_carry);
            }
            0x15 => {
                // DEC D 1 4 | Z 1 H -
                let is_half_carry = is_half_carry_sub_u8(self.cpu.get_d(), 1);
                let byte = self.cpu.get_d().wrapping_sub(1);

                self.cpu.set_d(byte);
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(true);
                self.cpu.set_fh(is_half_carry);
            }
            0x16 => {
                // LD D,d8 2 8 | - - - -
                let byte = self.read_op()?;
                self.cpu.set_d(byte);
            }
            0x17 => {
                // RLA 1 4 | 0 0 0 C
                unimplemented!("Opcode 0x17 (RLA 1 4) not implemented");
            }
            0x18 => {
                // JR r8 2 12 | - - - -
                unimplemented!("Opcode 0x18 (JR r8 2 12) not implemented");
            }
            0x19 => {
                // ADD HL,DE 1 8 | - 0 H C
                unimplemented!("Opcode 0x19 (ADD HL,DE 1 8) not implemented");
            }
            0x1A => {
                // LD A,(DE) 1 8 | - - - -
                let byte = self.mem.read(self.cpu.de)?;
                self.cpu.set_a(byte);
            }
            0x1B => {
                // DEC DE 1 8 | - - - -
                self.cpu.de = self.cpu.de.wrapping_sub(1);
            }
            0x1C => {
                // INC E 1 4 | Z 0 H -
                let is_half_carry = is_half_carry_add_u8(self.cpu.get_e(), 1);
                let byte = self.cpu.get_e().wrapping_add(1);

                self.cpu.set_e(byte);
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(false);
                self.cpu.set_fh(is_half_carry);
            }
            0x1D => {
                // DEC E 1 4 | Z 1 H -
                let is_half_carry = is_half_carry_sub_u8(self.cpu.get_e(), 1);
                let byte = self.cpu.get_e().wrapping_sub(1);

                self.cpu.set_e(byte);
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(true);
                self.cpu.set_fh(is_half_carry);
            }
            0x1E => {
                // LD E,d8 2 8 | - - - -
                let byte = self.read_op()?;
                self.cpu.set_e(byte);
            }
            0x1F => {
                // RRA 1 4 | 0 0 0 C
                unimplemented!("Opcode 0x1F (RRA 1 4) not implemented");
            }
            0x20 => {
                // JR NZ,r8 2 12/8 | - - - -
                unimplemented!("Opcode 0x20 (JR NZ,r8 2 12/8) not implemented");
            }
            0x21 => {
                // LD HL,d16 3 12 | - - - -
                let word = self.read_op_imm16()?;
                self.cpu.hl = word;
            }
            0x22 => {
                // LD (HL+),A 1 8 | - - - -
                let byte = self.cpu.get_a();
                self.mem.write(self.cpu.hl, byte)?;
                self.cpu.hl = self.cpu.hl.wrapping_add(1);
            }
            0x23 => {
                // INC HL 1 8 | - - - -
                self.cpu.hl = self.cpu.hl.wrapping_add(1);
            }
            0x24 => {
                // INC H 1 4 | Z 0 H -
                let is_half_carry = is_half_carry_add_u8(self.cpu.get_h(), 1);
                let byte = self.cpu.get_h().wrapping_add(1);

                self.cpu.set_h(byte);
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(false);
                self.cpu.set_fh(is_half_carry);
            }
            0x25 => {
                // DEC H 1 4 | Z 1 H -
                let is_half_carry = is_half_carry_sub_u8(self.cpu.get_h(), 1);
                let byte = self.cpu.get_h().wrapping_sub(1);

                self.cpu.set_h(byte);
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(true);
                self.cpu.set_fh(is_half_carry);
            }
            0x26 => {
                // LD H,d8 2 8 | - - - -
                let byte = self.read_op()?;
                self.cpu.set_h(byte);
            }
            0x27 => {
                // DAA 1 4 | Z - 0 C
                unimplemented!("Opcode 0x27 (DAA 1 4) not implemented");
            }
            0x28 => {
                // JR Z,r8 2 12/8 | - - - -
                unimplemented!("Opcode 0x28 (JR Z,r8 2 12/8) not implemented");
            }
            0x29 => {
                // ADD HL,HL 1 8 | - 0 H C
                unimplemented!("Opcode 0x29 (ADD HL,HL 1 8) not implemented");
            }
            0x2A => {
                // LD A,(HL+) 1 8 | - - - -
                let byte = self.mem.read(self.cpu.hl)?;
                self.cpu.set_a(byte);
                self.cpu.hl = self.cpu.hl.wrapping_add(1);
            }
            0x2B => {
                // DEC HL 1 8 | - - - -
                self.cpu.hl = self.cpu.hl.wrapping_sub(1);
            }
            0x2C => {
                // INC L 1 4 | Z 0 H -
                let is_half_carry = is_half_carry_add_u8(self.cpu.get_l(), 1);
                let byte = self.cpu.get_l().wrapping_add(1);

                self.cpu.set_l(byte);
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(false);
                self.cpu.set_fh(is_half_carry);
            }
            0x2D => {
                // DEC L 1 4 | Z 1 H -
                let is_half_carry = is_half_carry_sub_u8(self.cpu.get_l(), 1);
                let byte = self.cpu.get_l().wrapping_sub(1);

                self.cpu.set_l(byte);
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(true);
                self.cpu.set_fh(is_half_carry);
            }
            0x2E => {
                // LD L,d8 2 8 | - - - -
                let byte = self.read_op()?;
                self.cpu.set_l(byte);
            }
            0x2F => {
                // CPL 1 4 | - 1 1 -
                unimplemented!("Opcode 0x2F (CPL 1 4) not implemented");
            }
            0x30 => {
                // JR NC,r8 2 12/8 | - - - -
                unimplemented!("Opcode 0x30 (JR NC,r8 2 12/8) not implemented");
            }
            0x31 => {
                // LD SP,d16 3 12 | - - - -
                let word = self.read_op_imm16()?;
                self.cpu.sp = word;
            }
            0x32 => {
                // LD (HL-),A 1 8 | - - - -
                let byte = self.cpu.get_a();
                let word = self.cpu.hl;
                self.mem.write(word, byte)?;
                self.cpu.hl = self.cpu.hl.wrapping_sub(1);
            }
            0x33 => {
                // INC SP 1 8 | - - - -
                self.cpu.sp = self.cpu.sp.wrapping_add(1);
            }
            0x34 => {
                // INC (HL) 1 12 | Z 0 H -
                let byte = self.mem.read(self.cpu.hl)?;
                let is_half_carry = is_half_carry_add_u8(byte, 1);

                self.mem.write(self.cpu.hl, byte.wrapping_add(1))?;
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(false);
                self.cpu.set_fh(is_half_carry);
            }
            0x35 => {
                // DEC (HL) 1 12 | Z 1 H -
                let byte = self.mem.read(self.cpu.hl)?;
                let is_half_carry = is_half_carry_sub_u8(byte, 1);

                self.mem.write(self.cpu.hl, byte.wrapping_sub(1))?;
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(true);
                self.cpu.set_fh(is_half_carry);
            }
            0x36 => {
                // LD (HL),d8 2 12 | - - - -
                let byte = self.read_op()?;
                self.mem.write(self.cpu.hl, byte)?;
            }
            0x37 => {
                // SCF 1 4 | - 0 0 1
                unimplemented!("Opcode 0x37 (SCF 1 4) not implemented");
            }
            0x38 => {
                // JR C,r8 2 12/8 | - - - -
                unimplemented!("Opcode 0x38 (JR C,r8 2 12/8) not implemented");
            }
            0x39 => {
                // ADD HL,SP 1 8 | - 0 H C
                unimplemented!("Opcode 0x39 (ADD HL,SP 1 8) not implemented");
            }
            0x3A => {
                // LD A,(HL-) 1 8 | - - - -
                let byte = self.mem.read(self.cpu.hl)?;
                self.cpu.hl = self.cpu.hl.wrapping_sub(1);
                self.cpu.set_a(byte);
            }
            0x3B => {
                // DEC SP 1 8 | - - - -
                self.cpu.sp = self.cpu.sp.wrapping_sub(1);
            }
            0x3C => {
                // INC A 1 4 | Z 0 H -
                let is_half_carry = is_half_carry_add_u8(self.cpu.get_a(), 1);
                let byte = self.cpu.get_a().wrapping_add(1);

                self.cpu.set_a(byte);
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(false);
                self.cpu.set_fh(is_half_carry);
            }
            0x3D => {
                // DEC A 1 4 | Z 1 H -
                let is_half_carry = is_half_carry_sub_u8(self.cpu.get_a(), 1);
                let byte = self.cpu.get_a().wrapping_sub(1);

                self.cpu.set_a(byte);
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(true);
                self.cpu.set_fh(is_half_carry);
            }
            0x3E => {
                // LD A,d8 2 8 | - - - -
                let byte = self.read_op()?;
                self.cpu.set_a(byte);
            }
            0x3F => {
                // CCF 1 4 | - 0 0 C
                unimplemented!("Opcode 0x3F (CCF 1 4) not implemented");
            }
            0x40 => {
                // LD B,B 1 4 | - - - -
                let byte = self.cpu.get_b();
                self.cpu.set_b(byte);
            }
            0x41 => {
                // LD B,C 1 4 | - - - -
                let byte = self.cpu.get_c();
                self.cpu.set_b(byte);
            }
            0x42 => {
                // LD B,D 1 4 | - - - -
                let byte = self.cpu.get_d();
                self.cpu.set_b(byte);
            }
            0x43 => {
                // LD B,E 1 4 | - - - -
                let byte = self.cpu.get_e();
                self.cpu.set_b(byte);
            }
            0x44 => {
                // LD B,H 1 4 | - - - -
                let byte = self.cpu.get_h();
                self.cpu.set_b(byte);
            }
            0x45 => {
                // LD B,L 1 4 | - - - -
                let byte = self.cpu.get_l();
                self.cpu.set_b(byte);
            }
            0x46 => {
                // LD B,(HL) 1 8 | - - - -
                let byte = self.mem.read(self.cpu.hl)?;
                self.cpu.set_b(byte);
            }
            0x47 => {
                // LD B,A 1 4 | - - - -
                let byte = self.cpu.get_a();
                self.cpu.set_b(byte);
            }
            0x48 => {
                // LD C,B 1 4 | - - - -
                let byte = self.cpu.get_b();
                self.cpu.set_c(byte);
            }
            0x49 => {
                // LD C,C 1 4 | - - - -
                let byte = self.cpu.get_c();
                self.cpu.set_c(byte);
            }
            0x4A => {
                // LD C,D 1 4 | - - - -
                let byte = self.cpu.get_d();
                self.cpu.set_c(byte);
            }
            0x4B => {
                // LD C,E 1 4 | - - - -
                let byte = self.cpu.get_e();
                self.cpu.set_c(byte);
            }
            0x4C => {
                // LD C,H 1 4 | - - - -
                let byte = self.cpu.get_h();
                self.cpu.set_c(byte);
            }
            0x4D => {
                // LD C,L 1 4 | - - - -
                let byte = self.cpu.get_l();
                self.cpu.set_c(byte);
            }
            0x4E => {
                // LD C,(HL) 1 8 | - - - -
                let byte = self.mem.read(self.cpu.hl)?;
                self.cpu.set_c(byte);
            }
            0x4F => {
                // LD C,A 1 4 | - - - -
                let byte = self.cpu.get_a();
                self.cpu.set_c(byte);
            }
            0x50 => {
                // LD D,B 1 4 | - - - -
                let byte = self.cpu.get_b();
                self.cpu.set_d(byte);
            }
            0x51 => {
                // LD D,C 1 4 | - - - -
                let byte = self.cpu.get_c();
                self.cpu.set_d(byte);
            }
            0x52 => {
                // LD D,D 1 4 | - - - -
                let byte = self.cpu.get_d();
                self.cpu.set_d(byte);
            }
            0x53 => {
                // LD D,E 1 4 | - - - -
                let byte = self.cpu.get_e();
                self.cpu.set_d(byte);
            }
            0x54 => {
                // LD D,H 1 4 | - - - -
                let byte = self.cpu.get_h();
                self.cpu.set_d(byte);
            }
            0x55 => {
                // LD D,L 1 4 | - - - -
                let byte = self.cpu.get_l();
                self.cpu.set_d(byte);
            }
            0x56 => {
                // LD D,(HL) 1 8 | - - - -
                let byte = self.mem.read(self.cpu.hl)?;
                self.cpu.set_d(byte);
            }
            0x57 => {
                // LD D,A 1 4 | - - - -
                let byte = self.cpu.get_a();
                self.cpu.set_d(byte);
            }
            0x58 => {
                // LD E,B 1 4 | - - - -
                let byte = self.cpu.get_b();
                self.cpu.set_e(byte);
            }
            0x59 => {
                // LD E,C 1 4 | - - - -
                let byte = self.cpu.get_c();
                self.cpu.set_e(byte);
            }
            0x5A => {
                // LD E,D 1 4 | - - - -
                let byte = self.cpu.get_d();
                self.cpu.set_e(byte);
            }
            0x5B => {
                // LD E,E 1 4 | - - - -
                let byte = self.cpu.get_e();
                self.cpu.set_e(byte);
            }
            0x5C => {
                // LD E,H 1 4 | - - - -
                let byte = self.cpu.get_h();
                self.cpu.set_e(byte);
            }
            0x5D => {
                // LD E,L 1 4 | - - - -
                let byte = self.cpu.get_l();
                self.cpu.set_e(byte);
            }
            0x5E => {
                // LD E,(HL) 1 8 | - - - -
                let byte = self.mem.read(self.cpu.hl)?;
                self.cpu.set_e(byte);
            }
            0x5F => {
                // LD E,A 1 4 | - - - -
                let byte = self.cpu.get_a();
                self.cpu.set_e(byte);
            }
            0x60 => {
                // LD H,B 1 4 | - - - -
                let byte = self.cpu.get_b();
                self.cpu.set_h(byte);
            }
            0x61 => {
                // LD H,C 1 4 | - - - -
                let byte = self.cpu.get_c();
                self.cpu.set_h(byte);
            }
            0x62 => {
                // LD H,D 1 4 | - - - -
                let byte = self.cpu.get_d();
                self.cpu.set_h(byte);
            }
            0x63 => {
                // LD H,E 1 4 | - - - -
                let byte = self.cpu.get_e();
                self.cpu.set_h(byte);
            }
            0x64 => {
                // LD H,H 1 4 | - - - -
                let byte = self.cpu.get_h();
                self.cpu.set_h(byte);
            }
            0x65 => {
                // LD H,L 1 4 | - - - -
                let byte = self.cpu.get_l();
                self.cpu.set_h(byte);
            }
            0x66 => {
                // LD H,(HL) 1 8 | - - - -
                let byte = self.mem.read(self.cpu.hl)?;
                self.cpu.set_h(byte);
            }
            0x67 => {
                // LD H,A 1 4 | - - - -
                let byte = self.cpu.get_a();
                self.cpu.set_h(byte);
            }
            0x68 => {
                // LD L,B 1 4 | - - - -
                let byte = self.cpu.get_b();
                self.cpu.set_l(byte);
            }
            0x69 => {
                // LD L,C 1 4 | - - - -
                let byte = self.cpu.get_c();
                self.cpu.set_l(byte);
            }
            0x6A => {
                // LD L,D 1 4 | - - - -
                let byte = self.cpu.get_d();
                self.cpu.set_l(byte);
            }
            0x6B => {
                // LD L,E 1 4 | - - - -
                let byte = self.cpu.get_e();
                self.cpu.set_l(byte);
            }
            0x6C => {
                // LD L,H 1 4 | - - - -
                let byte = self.cpu.get_h();
                self.cpu.set_l(byte);
            }
            0x6D => {
                // LD L,L 1 4 | - - - -
                let byte = self.cpu.get_l();
                self.cpu.set_l(byte);
            }
            0x6E => {
                // LD L,(HL) 1 8 | - - - -
                let byte = self.mem.read(self.cpu.hl)?;
                self.cpu.set_l(byte);
            }
            0x6F => {
                // LD L,A 1 4 | - - - -
                let byte = self.cpu.get_a();
                self.cpu.set_l(byte);
            }
            0x70 => {
                // LD (HL),B 1 8 | - - - -
                let byte = self.cpu.get_b();
                self.mem.write(self.cpu.hl, byte)?;
            }
            0x71 => {
                // LD (HL),C 1 8 | - - - -
                let byte = self.cpu.get_c();
                self.mem.write(self.cpu.hl, byte)?;
            }
            0x72 => {
                // LD (HL),D 1 8 | - - - -
                let byte = self.cpu.get_d();
                self.mem.write(self.cpu.hl, byte)?;
            }
            0x73 => {
                // LD (HL),E 1 8 | - - - -
                let byte = self.cpu.get_e();
                self.mem.write(self.cpu.hl, byte)?;
            }
            0x74 => {
                // LD (HL),H 1 8 | - - - -
                let byte = self.cpu.get_h();
                self.mem.write(self.cpu.hl, byte)?;
            }
            0x75 => {
                // LD (HL),L 1 8 | - - - -
                let byte = self.cpu.get_l();
                self.mem.write(self.cpu.hl, byte)?;
            }
            0x76 => {
                // HALT 1 4 | - - - -
                unimplemented!("Opcode 0x76 (HALT 1 4) not implemented");
            }
            0x77 => {
                // LD (HL),A 1 8 | - - - -
                let byte = self.cpu.get_a();
                self.mem.write(self.cpu.hl, byte)?;
            }
            0x78 => {
                // LD A,B 1 4 | - - - -
                let byte = self.cpu.get_b();
                self.cpu.set_a(byte);
            }
            0x79 => {
                // LD A,C 1 4 | - - - -
                let byte = self.cpu.get_c();
                self.cpu.set_a(byte);
            }
            0x7A => {
                // LD A,D 1 4 | - - - -
                let byte = self.cpu.get_d();
                self.cpu.set_a(byte);
            }
            0x7B => {
                // LD A,E 1 4 | - - - -
                let byte = self.cpu.get_e();
                self.cpu.set_a(byte);
            }
            0x7C => {
                // LD A,H 1 4 | - - - -
                let byte = self.cpu.get_h();
                self.cpu.set_a(byte);
            }
            0x7D => {
                // LD A,L 1 4 | - - - -
                let byte = self.cpu.get_l();
                self.cpu.set_a(byte);
            }
            0x7E => {
                // LD A,(HL) 1 8 | - - - -
                let byte = self.mem.read(self.cpu.hl)?;
                self.cpu.set_a(byte);
            }
            0x7F => {
                // LD A,A 1 4 | - - - -
                let byte = self.cpu.get_a();
                self.cpu.set_a(byte);
            }
            0x80 => {
                // ADD A,B 1 4 | Z 0 H C
                let byte = self.cpu.get_b();
                self.cpu.add(byte);
            }
            0x81 => {
                // ADD A,C 1 4 | Z 0 H C
                let byte = self.cpu.get_c();
                self.cpu.add(byte);
            }
            0x82 => {
                // ADD A,D 1 4 | Z 0 H C
                let byte = self.cpu.get_d();
                self.cpu.add(byte);
            }
            0x83 => {
                // ADD A,E 1 4 | Z 0 H C
                let byte = self.cpu.get_e();
                self.cpu.add(byte);
            }
            0x84 => {
                // ADD A,H 1 4 | Z 0 H C
                let byte = self.cpu.get_h();
                self.cpu.add(byte);
            }
            0x85 => {
                // ADD A,L 1 4 | Z 0 H C
                let byte = self.cpu.get_l();
                self.cpu.add(byte);
            }
            0x86 => {
                // ADD A,(HL) 1 8 | Z 0 H C
                let byte = self.mem.read(self.cpu.hl)?;
                self.cpu.add(byte);
            }
            0x87 => {
                // ADD A,A 1 4 | Z 0 H C
                let byte = self.cpu.get_a();
                self.cpu.add(byte);
            }
            0x88 => {
                // ADC A,B 1 4 | Z 0 H C
                let byte = self.cpu.get_b().wrapping_add(self.cpu.get_fc());
                self.cpu.add(byte);
            }
            0x89 => {
                // ADC A,C 1 4 | Z 0 H C
                let byte = self.cpu.get_c().wrapping_add(self.cpu.get_fc());
                self.cpu.add(byte);
            }
            0x8A => {
                // ADC A,D 1 4 | Z 0 H C
                let byte = self.cpu.get_d().wrapping_add(self.cpu.get_fc());
                self.cpu.add(byte);
            }
            0x8B => {
                // ADC A,E 1 4 | Z 0 H C
                let byte = self.cpu.get_e().wrapping_add(self.cpu.get_fc());
                self.cpu.add(byte);
            }
            0x8C => {
                // ADC A,H 1 4 | Z 0 H C
                let byte = self.cpu.get_h().wrapping_add(self.cpu.get_fc());
                self.cpu.add(byte);
            }
            0x8D => {
                // ADC A,L 1 4 | Z 0 H C
                let byte = self.cpu.get_l().wrapping_add(self.cpu.get_fc());
                self.cpu.add(byte);
            }
            0x8E => {
                // ADC A,(HL) 1 8 | Z 0 H C
                let byte = self.mem.read(self.cpu.hl)?;
                self.cpu.add(byte);
            }
            0x8F => {
                // ADC A,A 1 4 | Z 0 H C
                let byte = self.cpu.get_a().wrapping_add(self.cpu.get_fc());
                self.cpu.add(byte);
            }
            0x90 => {
                // SUB B 1 4 | Z 1 H C
                let byte = self.cpu.get_b();
                self.cpu.sub(byte);
            }
            0x91 => {
                // SUB C 1 4 | Z 1 H C
                let byte = self.cpu.get_c();
                self.cpu.sub(byte);
            }
            0x92 => {
                // SUB D 1 4 | Z 1 H C
                let byte = self.cpu.get_d();
                self.cpu.sub(byte);
            }
            0x93 => {
                // SUB E 1 4 | Z 1 H C
                let byte = self.cpu.get_e();
                self.cpu.sub(byte);
            }
            0x94 => {
                // SUB H 1 4 | Z 1 H C
                let byte = self.cpu.get_h();
                self.cpu.sub(byte);
            }
            0x95 => {
                // SUB L 1 4 | Z 1 H C
                let byte = self.cpu.get_l();
                self.cpu.sub(byte);
            }
            0x96 => {
                // SUB (HL) 1 8 | Z 1 H C
                let byte = self.mem.read(self.cpu.hl)?;
                self.cpu.sub(byte);
            }
            0x97 => {
                // SUB A 1 4 | Z 1 H C
                let byte = self.cpu.get_a();
                self.cpu.sub(byte);
            }
            0x98 => {
                // SBC A,B 1 4 | Z 1 H C
                let byte = self.cpu.get_b().wrapping_sub(self.cpu.get_fc());
                self.cpu.sub(byte);
            }
            0x99 => {
                // SBC A,C 1 4 | Z 1 H C
                let byte = self.cpu.get_c().wrapping_sub(self.cpu.get_fc());
                self.cpu.sub(byte);
            }
            0x9A => {
                // SBC A,D 1 4 | Z 1 H C
                let byte = self.cpu.get_d().wrapping_sub(self.cpu.get_fc());
                self.cpu.sub(byte);
            }
            0x9B => {
                // SBC A,E 1 4 | Z 1 H C
                let byte = self.cpu.get_e().wrapping_sub(self.cpu.get_fc());
                self.cpu.sub(byte);
            }
            0x9C => {
                // SBC A,H 1 4 | Z 1 H C
                let byte = self.cpu.get_h().wrapping_sub(self.cpu.get_fc());
                self.cpu.sub(byte);
            }
            0x9D => {
                // SBC A,L 1 4 | Z 1 H C
                let byte = self.cpu.get_l().wrapping_sub(self.cpu.get_fc());
                self.cpu.sub(byte);
            }
            0x9E => {
                // SBC A,(HL) 1 8 | Z 1 H C
                let byte = self.mem.read(self.cpu.hl)?;
                self.cpu.sub(byte);
            }
            0x9F => {
                // SBC A,A 1 4 | Z 1 H C
                let byte = self.cpu.get_a().wrapping_sub(self.cpu.get_fc());
                self.cpu.sub(byte);
            }
            0xA0 => {
                // AND B 1 4 | Z 0 1 0
                let byte = self.cpu.get_b();
                self.cpu.and(byte);
            }
            0xA1 => {
                // AND C 1 4 | Z 0 1 0
                let byte = self.cpu.get_c();
                self.cpu.and(byte);
            }
            0xA2 => {
                // AND D 1 4 | Z 0 1 0
                let byte = self.cpu.get_d();
                self.cpu.and(byte);
            }
            0xA3 => {
                // AND E 1 4 | Z 0 1 0
                let byte = self.cpu.get_e();
                self.cpu.and(byte);
            }
            0xA4 => {
                // AND H 1 4 | Z 0 1 0
                let byte = self.cpu.get_h();
                self.cpu.and(byte);
            }
            0xA5 => {
                // AND L 1 4 | Z 0 1 0
                let byte = self.cpu.get_l();
                self.cpu.and(byte);
            }
            0xA6 => {
                // AND (HL) 1 8 | Z 0 1 0
                let byte = self.mem.read(self.cpu.hl)?;
                self.cpu.and(byte);
            }
            0xA7 => {
                // AND A 1 4 | Z 0 1 0
                let byte = self.cpu.get_a();
                self.cpu.and(byte);
            }
            0xA8 => {
                // XOR B 1 4 | Z 0 0 0
                let byte = self.cpu.get_b();
                self.cpu.xor(byte);
            }
            0xA9 => {
                // XOR C 1 4 | Z 0 0 0
                let byte = self.cpu.get_c();
                self.cpu.xor(byte);
            }
            0xAA => {
                // XOR D 1 4 | Z 0 0 0
                let byte = self.cpu.get_d();
                self.cpu.xor(byte);
            }
            0xAB => {
                // XOR E 1 4 | Z 0 0 0
                let byte = self.cpu.get_e();
                self.cpu.xor(byte);
            }
            0xAC => {
                // XOR H 1 4 | Z 0 0 0
                let byte = self.cpu.get_h();
                self.cpu.xor(byte);
            }
            0xAD => {
                // XOR L 1 4 | Z 0 0 0
                let byte = self.cpu.get_l();
                self.cpu.xor(byte);
            }
            0xAE => {
                // XOR (HL) 1 8 | Z 0 0 0
                let byte = self.mem.read(self.cpu.hl)?;
                self.cpu.xor(byte);
            }
            0xAF => {
                // XOR A 1 4 | Z 0 0 0
                let byte = self.cpu.get_a();
                self.cpu.xor(byte);
            }
            0xB0 => {
                // OR B 1 4 | Z 0 0 0
                let byte = self.cpu.get_b();
                self.cpu.or(byte);
            }
            0xB1 => {
                // OR C 1 4 | Z 0 0 0
                let byte = self.cpu.get_c();
                self.cpu.or(byte);
            }
            0xB2 => {
                // OR D 1 4 | Z 0 0 0
                let byte = self.cpu.get_d();
                self.cpu.or(byte);
            }
            0xB3 => {
                // OR E 1 4 | Z 0 0 0
                let byte = self.cpu.get_e();
                self.cpu.or(byte);
            }
            0xB4 => {
                // OR H 1 4 | Z 0 0 0
                let byte = self.cpu.get_h();
                self.cpu.or(byte);
            }
            0xB5 => {
                // OR L 1 4 | Z 0 0 0
                let byte = self.cpu.get_l();
                self.cpu.or(byte);
            }
            0xB6 => {
                // OR (HL) 1 8 | Z 0 0 0
                let byte = self.mem.read(self.cpu.hl)?;
                self.cpu.or(byte);
            }
            0xB7 => {
                // OR A 1 4 | Z 0 0 0
                let byte = self.cpu.get_a();
                self.cpu.or(byte);
            }
            0xB8 => {
                // CP B 1 4 | Z 1 H C
                let byte = self.cpu.get_b();
                self.cpu.cp(byte);
            }
            0xB9 => {
                // CP C 1 4 | Z 1 H C
                let byte = self.cpu.get_c();
                self.cpu.cp(byte);
            }
            0xBA => {
                // CP D 1 4 | Z 1 H C
                let byte = self.cpu.get_d();
                self.cpu.cp(byte);
            }
            0xBB => {
                // CP E 1 4 | Z 1 H C
                let byte = self.cpu.get_e();
                self.cpu.cp(byte);
            }
            0xBC => {
                // CP H 1 4 | Z 1 H C
                let byte = self.cpu.get_h();
                self.cpu.cp(byte);
            }
            0xBD => {
                // CP L 1 4 | Z 1 H C
                let byte = self.cpu.get_l();
                self.cpu.cp(byte);
            }
            0xBE => {
                // CP (HL) 1 8 | Z 1 H C
                let byte = self.mem.read(self.cpu.hl)?;
                self.cpu.cp(byte);
            }
            0xBF => {
                // CP A 1 4 | Z 1 H C
                let byte = self.cpu.get_a();
                self.cpu.cp(byte);
            }
            0xC0 => {
                // RET NZ 1 20/8 | - - - -
                unimplemented!("Opcode 0xC0 (RET NZ 1 20/8) not implemented");
            }
            0xC1 => {
                // POP BC 1 12 | - - - -
                self.mem.read_u16(self.cpu.sp)?;
                self.cpu.bc = self.cpu.sp.wrapping_add(2);
            }
            0xC2 => {
                // JP NZ,a16 3 16/12 | - - - -
                unimplemented!("Opcode 0xC2 (JP NZ,a16 3 16/12) not implemented");
            }
            0xC3 => {
                // JP a16 3 16 | - - - -
                unimplemented!("Opcode 0xC3 (JP a16 3 16) not implemented");
            }
            0xC4 => {
                // CALL NZ,a16 3 24/12 | - - - -
                unimplemented!("Opcode 0xC4 (CALL NZ,a16 3 24/12) not implemented");
            }
            0xC5 => {
                // PUSH BC 1 16 | - - - -
                self.cpu.sp = self.cpu.sp.wrapping_sub(2);
                self.mem.write_u16(self.cpu.sp, self.cpu.bc)?;
            }
            0xC6 => {
                // ADD A,d8 2 8 | Z 0 H C
                let byte = self.read_op()?;
                self.cpu.add(byte);
            }
            0xC7 => {
                // RST 00H 1 16 | - - - -
                unimplemented!("Opcode 0xC7 (RST 00H 1 16) not implemented");
            }
            0xC8 => {
                // RET Z 1 20/8 | - - - -
                unimplemented!("Opcode 0xC8 (RET Z 1 20/8) not implemented");
            }
            0xC9 => {
                // RET 1 16 | - - - -
                unimplemented!("Opcode 0xC9 (RET 1 16) not implemented");
            }
            0xCA => {
                // JP Z,a16 3 16/12 | - - - -
                unimplemented!("Opcode 0xCA (JP Z,a16 3 16/12) not implemented");
            }
            0xCB => {
                // PREFIX CB 1 4 | - - - -

                let op_cb = self.read_op()?;

                log::debug!(
                    ">>(cb) {} (0x{:2X?})",
                    opcode_cb_name[op_cb as usize],
                    op_cb
                );

                match op_cb {
                    0x00 => {
                        // RLC B 2 8F | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x00 (RLC B 2 8F) not implemented");
                    }
                    0x01 => {
                        // RLC C 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x01 (RLC C 2 8) not implemented");
                    }
                    0x02 => {
                        // RLC D 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x02 (RLC D 2 8) not implemented");
                    }
                    0x03 => {
                        // RLC E 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x03 (RLC E 2 8) not implemented");
                    }
                    0x04 => {
                        // RLC H 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x04 (RLC H 2 8) not implemented");
                    }
                    0x05 => {
                        // RLC L 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x05 (RLC L 2 8) not implemented");
                    }
                    0x06 => {
                        // RLC (HL) 2 16 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x06 (RLC (HL) 2 16) not implemented");
                    }
                    0x07 => {
                        // RLC A 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x07 (RLC A 2 8) not implemented");
                    }
                    0x08 => {
                        // RRC B 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x08 (RRC B 2 8) not implemented");
                    }
                    0x09 => {
                        // RRC C 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x09 (RRC C 2 8) not implemented");
                    }
                    0x0A => {
                        // RRC D 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x0A (RRC D 2 8) not implemented");
                    }
                    0x0B => {
                        // RRC E 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x0B (RRC E 2 8) not implemented");
                    }
                    0x0C => {
                        // RRC H 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x0C (RRC H 2 8) not implemented");
                    }
                    0x0D => {
                        // RRC L 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x0D (RRC L 2 8) not implemented");
                    }
                    0x0E => {
                        // RRC (HL) 2 16 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x0E (RRC (HL) 2 16) not implemented");
                    }
                    0x0F => {
                        // RRC A 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x0F (RRC A 2 8) not implemented");
                    }
                    0x10 => {
                        // RL B 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x10 (RL B 2 8) not implemented");
                    }
                    0x11 => {
                        // RL C 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x11 (RL C 2 8) not implemented");
                    }
                    0x12 => {
                        // RL D 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x12 (RL D 2 8) not implemented");
                    }
                    0x13 => {
                        // RL E 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x13 (RL E 2 8) not implemented");
                    }
                    0x14 => {
                        // RL H 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x14 (RL H 2 8) not implemented");
                    }
                    0x15 => {
                        // RL L 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x15 (RL L 2 8) not implemented");
                    }
                    0x16 => {
                        // RL (HL) 2 16 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x16 (RL (HL) 2 16) not implemented");
                    }
                    0x17 => {
                        // RL A 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x17 (RL A 2 8) not implemented");
                    }
                    0x18 => {
                        // RR B 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x18 (RR B 2 8) not implemented");
                    }
                    0x19 => {
                        // RR C 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x19 (RR C 2 8) not implemented");
                    }
                    0x1A => {
                        // RR D 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x1A (RR D 2 8) not implemented");
                    }
                    0x1B => {
                        // RR E 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x1B (RR E 2 8) not implemented");
                    }
                    0x1C => {
                        // RR H 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x1C (RR H 2 8) not implemented");
                    }
                    0x1D => {
                        // RR L 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x1D (RR L 2 8) not implemented");
                    }
                    0x1E => {
                        // RR (HL) 2 16 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x1E (RR (HL) 2 16) not implemented");
                    }
                    0x1F => {
                        // RR A 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x1F (RR A 2 8) not implemented");
                    }
                    0x20 => {
                        // SLA B 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x20 (SLA B 2 8) not implemented");
                    }
                    0x21 => {
                        // SLA C 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x21 (SLA C 2 8) not implemented");
                    }
                    0x22 => {
                        // SLA D 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x22 (SLA D 2 8) not implemented");
                    }
                    0x23 => {
                        // SLA E 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x23 (SLA E 2 8) not implemented");
                    }
                    0x24 => {
                        // SLA H 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x24 (SLA H 2 8) not implemented");
                    }
                    0x25 => {
                        // SLA L 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x25 (SLA L 2 8) not implemented");
                    }
                    0x26 => {
                        // SLA (HL) 2 16 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x26 (SLA (HL) 2 16) not implemented");
                    }
                    0x27 => {
                        // SLA A 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x27 (SLA A 2 8) not implemented");
                    }
                    0x28 => {
                        // SRA B 2 8 | Z 0 0 0
                        unimplemented!("Prefix CB opcode 0x28 (SRA B 2 8) not implemented");
                    }
                    0x29 => {
                        // SRA C 2 8 | Z 0 0 0
                        unimplemented!("Prefix CB opcode 0x29 (SRA C 2 8) not implemented");
                    }
                    0x2A => {
                        // SRA D 2 8 | Z 0 0 0
                        unimplemented!("Prefix CB opcode 0x2A (SRA D 2 8) not implemented");
                    }
                    0x2B => {
                        // SRA E 2 8 | Z 0 0 0
                        unimplemented!("Prefix CB opcode 0x2B (SRA E 2 8) not implemented");
                    }
                    0x2C => {
                        // SRA H 2 8 | Z 0 0 0
                        unimplemented!("Prefix CB opcode 0x2C (SRA H 2 8) not implemented");
                    }
                    0x2D => {
                        // SRA L 2 8 | Z 0 0 0
                        unimplemented!("Prefix CB opcode 0x2D (SRA L 2 8) not implemented");
                    }
                    0x2E => {
                        // SRA (HL) 2 16 | Z 0 0 0
                        unimplemented!("Prefix CB opcode 0x2E (SRA (HL) 2 16) not implemented");
                    }
                    0x2F => {
                        // SRA A 2 8 | Z 0 0 0
                        unimplemented!("Prefix CB opcode 0x2F (SRA A 2 8) not implemented");
                    }
                    0x30 => {
                        // SWAP B 2 8 | Z 0 0 0
                        unimplemented!("Prefix CB opcode 0x30 (SWAP B 2 8) not implemented");
                    }
                    0x31 => {
                        // SWAP C 2 8 | Z 0 0 0
                        unimplemented!("Prefix CB opcode 0x31 (SWAP C 2 8) not implemented");
                    }
                    0x32 => {
                        // SWAP D 2 8 | Z 0 0 0
                        unimplemented!("Prefix CB opcode 0x32 (SWAP D 2 8) not implemented");
                    }
                    0x33 => {
                        // SWAP E 2 8 | Z 0 0 0
                        unimplemented!("Prefix CB opcode 0x33 (SWAP E 2 8) not implemented");
                    }
                    0x34 => {
                        // SWAP H 2 8 | Z 0 0 0
                        unimplemented!("Prefix CB opcode 0x34 (SWAP H 2 8) not implemented");
                    }
                    0x35 => {
                        // SWAP L 2 8 | Z 0 0 0
                        unimplemented!("Prefix CB opcode 0x35 (SWAP L 2 8) not implemented");
                    }
                    0x36 => {
                        // SWAP (HL) 2 16 | Z 0 0 0
                        unimplemented!("Prefix CB opcode 0x36 (SWAP (HL) 2 16) not implemented");
                    }
                    0x37 => {
                        // SWAP A 2 8 | Z 0 0 0
                        unimplemented!("Prefix CB opcode 0x37 (SWAP A 2 8) not implemented");
                    }
                    0x38 => {
                        // SRL B 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x38 (SRL B 2 8) not implemented");
                    }
                    0x39 => {
                        // SRL C 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x39 (SRL C 2 8) not implemented");
                    }
                    0x3A => {
                        // SRL D 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x3A (SRL D 2 8) not implemented");
                    }
                    0x3B => {
                        // SRL E 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x3B (SRL E 2 8) not implemented");
                    }
                    0x3C => {
                        // SRL H 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x3C (SRL H 2 8) not implemented");
                    }
                    0x3D => {
                        // SRL L 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x3D (SRL L 2 8) not implemented");
                    }
                    0x3E => {
                        // SRL (HL) 2 16 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x3E (SRL (HL) 2 16) not implemented");
                    }
                    0x3F => {
                        // SRL A 2 8 | Z 0 0 C
                        unimplemented!("Prefix CB opcode 0x3F (SRL A 2 8) not implemented");
                    }
                    0x40 => {
                        // BIT 0,B 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x40 (BIT 0,B 2 8) not implemented");
                    }
                    0x41 => {
                        // BIT 0,C 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x41 (BIT 0,C 2 8) not implemented");
                    }
                    0x42 => {
                        // BIT 0,D 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x42 (BIT 0,D 2 8) not implemented");
                    }
                    0x43 => {
                        // BIT 0,E 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x43 (BIT 0,E 2 8) not implemented");
                    }
                    0x44 => {
                        // BIT 0,H 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x44 (BIT 0,H 2 8) not implemented");
                    }
                    0x45 => {
                        // BIT 0,L 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x45 (BIT 0,L 2 8) not implemented");
                    }
                    0x46 => {
                        // BIT 0,(HL) 2 16 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x46 (BIT 0,(HL) 2 16) not implemented");
                    }
                    0x47 => {
                        // BIT 0,A 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x47 (BIT 0,A 2 8) not implemented");
                    }
                    0x48 => {
                        // BIT 1,B 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x48 (BIT 1,B 2 8) not implemented");
                    }
                    0x49 => {
                        // BIT 1,C 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x49 (BIT 1,C 2 8) not implemented");
                    }
                    0x4A => {
                        // BIT 1,D 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x4A (BIT 1,D 2 8) not implemented");
                    }
                    0x4B => {
                        // BIT 1,E 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x4B (BIT 1,E 2 8) not implemented");
                    }
                    0x4C => {
                        // BIT 1,H 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x4C (BIT 1,H 2 8) not implemented");
                    }
                    0x4D => {
                        // BIT 1,L 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x4D (BIT 1,L 2 8) not implemented");
                    }
                    0x4E => {
                        // BIT 1,(HL) 2 16 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x4E (BIT 1,(HL) 2 16) not implemented");
                    }
                    0x4F => {
                        // BIT 1,A 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x4F (BIT 1,A 2 8) not implemented");
                    }
                    0x50 => {
                        // BIT 2,B 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x50 (BIT 2,B 2 8) not implemented");
                    }
                    0x51 => {
                        // BIT 2,C 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x51 (BIT 2,C 2 8) not implemented");
                    }
                    0x52 => {
                        // BIT 2,D 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x52 (BIT 2,D 2 8) not implemented");
                    }
                    0x53 => {
                        // BIT 2,E 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x53 (BIT 2,E 2 8) not implemented");
                    }
                    0x54 => {
                        // BIT 2,H 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x54 (BIT 2,H 2 8) not implemented");
                    }
                    0x55 => {
                        // BIT 2,L 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x55 (BIT 2,L 2 8) not implemented");
                    }
                    0x56 => {
                        // BIT 2,(HL) 2 16 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x56 (BIT 2,(HL) 2 16) not implemented");
                    }
                    0x57 => {
                        // BIT 2,A 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x57 (BIT 2,A 2 8) not implemented");
                    }
                    0x58 => {
                        // BIT 3,B 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x58 (BIT 3,B 2 8) not implemented");
                    }
                    0x59 => {
                        // BIT 3,C 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x59 (BIT 3,C 2 8) not implemented");
                    }
                    0x5A => {
                        // BIT 3,D 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x5A (BIT 3,D 2 8) not implemented");
                    }
                    0x5B => {
                        // BIT 3,E 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x5B (BIT 3,E 2 8) not implemented");
                    }
                    0x5C => {
                        // BIT 3,H 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x5C (BIT 3,H 2 8) not implemented");
                    }
                    0x5D => {
                        // BIT 3,L 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x5D (BIT 3,L 2 8) not implemented");
                    }
                    0x5E => {
                        // BIT 3,(HL) 2 16 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x5E (BIT 3,(HL) 2 16) not implemented");
                    }
                    0x5F => {
                        // BIT 3,A 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x5F (BIT 3,A 2 8) not implemented");
                    }
                    0x60 => {
                        // BIT 4,B 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x60 (BIT 4,B 2 8) not implemented");
                    }
                    0x61 => {
                        // BIT 4,C 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x61 (BIT 4,C 2 8) not implemented");
                    }
                    0x62 => {
                        // BIT 4,D 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x62 (BIT 4,D 2 8) not implemented");
                    }
                    0x63 => {
                        // BIT 4,E 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x63 (BIT 4,E 2 8) not implemented");
                    }
                    0x64 => {
                        // BIT 4,H 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x64 (BIT 4,H 2 8) not implemented");
                    }
                    0x65 => {
                        // BIT 4,L 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x65 (BIT 4,L 2 8) not implemented");
                    }
                    0x66 => {
                        // BIT 4,(HL) 2 16 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x66 (BIT 4,(HL) 2 16) not implemented");
                    }
                    0x67 => {
                        // BIT 4,A 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x67 (BIT 4,A 2 8) not implemented");
                    }
                    0x68 => {
                        // BIT 5,B 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x68 (BIT 5,B 2 8) not implemented");
                    }
                    0x69 => {
                        // BIT 5,C 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x69 (BIT 5,C 2 8) not implemented");
                    }
                    0x6A => {
                        // BIT 5,D 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x6A (BIT 5,D 2 8) not implemented");
                    }
                    0x6B => {
                        // BIT 5,E 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x6B (BIT 5,E 2 8) not implemented");
                    }
                    0x6C => {
                        // BIT 5,H 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x6C (BIT 5,H 2 8) not implemented");
                    }
                    0x6D => {
                        // BIT 5,L 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x6D (BIT 5,L 2 8) not implemented");
                    }
                    0x6E => {
                        // BIT 5,(HL) 2 16 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x6E (BIT 5,(HL) 2 16) not implemented");
                    }
                    0x6F => {
                        // BIT 5,A 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x6F (BIT 5,A 2 8) not implemented");
                    }
                    0x70 => {
                        // BIT 6,B 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x70 (BIT 6,B 2 8) not implemented");
                    }
                    0x71 => {
                        // BIT 6,C 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x71 (BIT 6,C 2 8) not implemented");
                    }
                    0x72 => {
                        // BIT 6,D 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x72 (BIT 6,D 2 8) not implemented");
                    }
                    0x73 => {
                        // BIT 6,E 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x73 (BIT 6,E 2 8) not implemented");
                    }
                    0x74 => {
                        // BIT 6,H 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x74 (BIT 6,H 2 8) not implemented");
                    }
                    0x75 => {
                        // BIT 6,L 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x75 (BIT 6,L 2 8) not implemented");
                    }
                    0x76 => {
                        // BIT 6,(HL) 2 16 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x76 (BIT 6,(HL) 2 16) not implemented");
                    }
                    0x77 => {
                        // BIT 6,A 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x77 (BIT 6,A 2 8) not implemented");
                    }
                    0x78 => {
                        // BIT 7,B 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x78 (BIT 7,B 2 8) not implemented");
                    }
                    0x79 => {
                        // BIT 7,C 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x79 (BIT 7,C 2 8) not implemented");
                    }
                    0x7A => {
                        // BIT 7,D 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x7A (BIT 7,D 2 8) not implemented");
                    }
                    0x7B => {
                        // BIT 7,E 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x7B (BIT 7,E 2 8) not implemented");
                    }
                    0x7C => {
                        // BIT 7,H 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x7C (BIT 7,H 2 8) not implemented");
                    }
                    0x7D => {
                        // BIT 7,L 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x7D (BIT 7,L 2 8) not implemented");
                    }
                    0x7E => {
                        // BIT 7,(HL) 2 16 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x7E (BIT 7,(HL) 2 16) not implemented");
                    }
                    0x7F => {
                        // BIT 7,A 2 8 | Z 0 1 -
                        unimplemented!("Prefix CB opcode 0x7F (BIT 7,A 2 8) not implemented");
                    }
                    0x80 => {
                        // RES 0,B 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x80 (RES 0,B 2 8) not implemented");
                    }
                    0x81 => {
                        // RES 0,C 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x81 (RES 0,C 2 8) not implemented");
                    }
                    0x82 => {
                        // RES 0,D 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x82 (RES 0,D 2 8) not implemented");
                    }
                    0x83 => {
                        // RES 0,E 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x83 (RES 0,E 2 8) not implemented");
                    }
                    0x84 => {
                        // RES 0,H 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x84 (RES 0,H 2 8) not implemented");
                    }
                    0x85 => {
                        // RES 0,L 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x85 (RES 0,L 2 8) not implemented");
                    }
                    0x86 => {
                        // RES 0,(HL) 2 16 | - - - -
                        unimplemented!("Prefix CB opcode 0x86 (RES 0,(HL) 2 16) not implemented");
                    }
                    0x87 => {
                        // RES 0,A 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x87 (RES 0,A 2 8) not implemented");
                    }
                    0x88 => {
                        // RES 1,B 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x88 (RES 1,B 2 8) not implemented");
                    }
                    0x89 => {
                        // RES 1,C 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x89 (RES 1,C 2 8) not implemented");
                    }
                    0x8A => {
                        // RES 1,D 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x8A (RES 1,D 2 8) not implemented");
                    }
                    0x8B => {
                        // RES 1,E 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x8B (RES 1,E 2 8) not implemented");
                    }
                    0x8C => {
                        // RES 1,H 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x8C (RES 1,H 2 8) not implemented");
                    }
                    0x8D => {
                        // RES 1,L 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x8D (RES 1,L 2 8) not implemented");
                    }
                    0x8E => {
                        // RES 1,(HL) 2 16 | - - - -
                        unimplemented!("Prefix CB opcode 0x8E (RES 1,(HL) 2 16) not implemented");
                    }
                    0x8F => {
                        // RES 1,A 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x8F (RES 1,A 2 8) not implemented");
                    }
                    0x90 => {
                        // RES 2,B 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x90 (RES 2,B 2 8) not implemented");
                    }
                    0x91 => {
                        // RES 2,C 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x91 (RES 2,C 2 8) not implemented");
                    }
                    0x92 => {
                        // RES 2,D 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x92 (RES 2,D 2 8) not implemented");
                    }
                    0x93 => {
                        // RES 2,E 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x93 (RES 2,E 2 8) not implemented");
                    }
                    0x94 => {
                        // RES 2,H 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x94 (RES 2,H 2 8) not implemented");
                    }
                    0x95 => {
                        // RES 2,L 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x95 (RES 2,L 2 8) not implemented");
                    }
                    0x96 => {
                        // RES 2,(HL) 2 16 | - - - -
                        unimplemented!("Prefix CB opcode 0x96 (RES 2,(HL) 2 16) not implemented");
                    }
                    0x97 => {
                        // RES 2,A 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x97 (RES 2,A 2 8) not implemented");
                    }
                    0x98 => {
                        // RES 3,B 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x98 (RES 3,B 2 8) not implemented");
                    }
                    0x99 => {
                        // RES 3,C 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x99 (RES 3,C 2 8) not implemented");
                    }
                    0x9A => {
                        // RES 3,D 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x9A (RES 3,D 2 8) not implemented");
                    }
                    0x9B => {
                        // RES 3,E 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x9B (RES 3,E 2 8) not implemented");
                    }
                    0x9C => {
                        // RES 3,H 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x9C (RES 3,H 2 8) not implemented");
                    }
                    0x9D => {
                        // RES 3,L 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x9D (RES 3,L 2 8) not implemented");
                    }
                    0x9E => {
                        // RES 3,(HL) 2 16 | - - - -
                        unimplemented!("Prefix CB opcode 0x9E (RES 3,(HL) 2 16) not implemented");
                    }
                    0x9F => {
                        // RES 3,A 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0x9F (RES 3,A 2 8) not implemented");
                    }
                    0xA0 => {
                        // RES 4,B 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xA0 (RES 4,B 2 8) not implemented");
                    }
                    0xA1 => {
                        // RES 4,C 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xA1 (RES 4,C 2 8) not implemented");
                    }
                    0xA2 => {
                        // RES 4,D 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xA2 (RES 4,D 2 8) not implemented");
                    }
                    0xA3 => {
                        // RES 4,E 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xA3 (RES 4,E 2 8) not implemented");
                    }
                    0xA4 => {
                        // RES 4,H 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xA4 (RES 4,H 2 8) not implemented");
                    }
                    0xA5 => {
                        // RES 4,L 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xA5 (RES 4,L 2 8) not implemented");
                    }
                    0xA6 => {
                        // RES 4,(HL) 2 16 | - - - -
                        unimplemented!("Prefix CB opcode 0xA6 (RES 4,(HL) 2 16) not implemented");
                    }
                    0xA7 => {
                        // RES 4,A 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xA7 (RES 4,A 2 8) not implemented");
                    }
                    0xA8 => {
                        // RES 5,B 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xA8 (RES 5,B 2 8) not implemented");
                    }
                    0xA9 => {
                        // RES 5,C 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xA9 (RES 5,C 2 8) not implemented");
                    }
                    0xAA => {
                        // RES 5,D 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xAA (RES 5,D 2 8) not implemented");
                    }
                    0xAB => {
                        // RES 5,E 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xAB (RES 5,E 2 8) not implemented");
                    }
                    0xAC => {
                        // RES 5,H 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xAC (RES 5,H 2 8) not implemented");
                    }
                    0xAD => {
                        // RES 5,L 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xAD (RES 5,L 2 8) not implemented");
                    }
                    0xAE => {
                        // RES 5,(HL) 2 16 | - - - -
                        unimplemented!("Prefix CB opcode 0xAE (RES 5,(HL) 2 16) not implemented");
                    }
                    0xAF => {
                        // RES 5,A 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xAF (RES 5,A 2 8) not implemented");
                    }
                    0xB0 => {
                        // RES 6,B 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xB0 (RES 6,B 2 8) not implemented");
                    }
                    0xB1 => {
                        // RES 6,C 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xB1 (RES 6,C 2 8) not implemented");
                    }
                    0xB2 => {
                        // RES 6,D 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xB2 (RES 6,D 2 8) not implemented");
                    }
                    0xB3 => {
                        // RES 6,E 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xB3 (RES 6,E 2 8) not implemented");
                    }
                    0xB4 => {
                        // RES 6,H 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xB4 (RES 6,H 2 8) not implemented");
                    }
                    0xB5 => {
                        // RES 6,L 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xB5 (RES 6,L 2 8) not implemented");
                    }
                    0xB6 => {
                        // RES 6,(HL) 2 16 | - - - -
                        unimplemented!("Prefix CB opcode 0xB6 (RES 6,(HL) 2 16) not implemented");
                    }
                    0xB7 => {
                        // RES 6,A 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xB7 (RES 6,A 2 8) not implemented");
                    }
                    0xB8 => {
                        // RES 7,B 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xB8 (RES 7,B 2 8) not implemented");
                    }
                    0xB9 => {
                        // RES 7,C 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xB9 (RES 7,C 2 8) not implemented");
                    }
                    0xBA => {
                        // RES 7,D 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xBA (RES 7,D 2 8) not implemented");
                    }
                    0xBB => {
                        // RES 7,E 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xBB (RES 7,E 2 8) not implemented");
                    }
                    0xBC => {
                        // RES 7,H 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xBC (RES 7,H 2 8) not implemented");
                    }
                    0xBD => {
                        // RES 7,L 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xBD (RES 7,L 2 8) not implemented");
                    }
                    0xBE => {
                        // RES 7,(HL) 2 16 | - - - -
                        unimplemented!("Prefix CB opcode 0xBE (RES 7,(HL) 2 16) not implemented");
                    }
                    0xBF => {
                        // RES 7,A 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xBF (RES 7,A 2 8) not implemented");
                    }
                    0xC0 => {
                        // SET 0,B 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xC0 (SET 0,B 2 8) not implemented");
                    }
                    0xC1 => {
                        // SET 0,C 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xC1 (SET 0,C 2 8) not implemented");
                    }
                    0xC2 => {
                        // SET 0,D 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xC2 (SET 0,D 2 8) not implemented");
                    }
                    0xC3 => {
                        // SET 0,E 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xC3 (SET 0,E 2 8) not implemented");
                    }
                    0xC4 => {
                        // SET 0,H 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xC4 (SET 0,H 2 8) not implemented");
                    }
                    0xC5 => {
                        // SET 0,L 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xC5 (SET 0,L 2 8) not implemented");
                    }
                    0xC6 => {
                        // SET 0,(HL) 2 16 | - - - -
                        unimplemented!("Prefix CB opcode 0xC6 (SET 0,(HL) 2 16) not implemented");
                    }
                    0xC7 => {
                        // SET 0,A 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xC7 (SET 0,A 2 8) not implemented");
                    }
                    0xC8 => {
                        // SET 1,B 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xC8 (SET 1,B 2 8) not implemented");
                    }
                    0xC9 => {
                        // SET 1,C 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xC9 (SET 1,C 2 8) not implemented");
                    }
                    0xCA => {
                        // SET 1,D 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xCA (SET 1,D 2 8) not implemented");
                    }
                    0xCB => {
                        // SET 1,E 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xCB (SET 1,E 2 8) not implemented");
                    }
                    0xCC => {
                        // SET 1,H 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xCC (SET 1,H 2 8) not implemented");
                    }
                    0xCD => {
                        // SET 1,L 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xCD (SET 1,L 2 8) not implemented");
                    }
                    0xCE => {
                        // SET 1,(HL) 2 16 | - - - -
                        unimplemented!("Prefix CB opcode 0xCE (SET 1,(HL) 2 16) not implemented");
                    }
                    0xCF => {
                        // SET 1,A 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xCF (SET 1,A 2 8) not implemented");
                    }
                    0xD0 => {
                        // SET 2,B 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xD0 (SET 2,B 2 8) not implemented");
                    }
                    0xD1 => {
                        // SET 2,C 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xD1 (SET 2,C 2 8) not implemented");
                    }
                    0xD2 => {
                        // SET 2,D 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xD2 (SET 2,D 2 8) not implemented");
                    }
                    0xD3 => {
                        // SET 2,E 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xD3 (SET 2,E 2 8) not implemented");
                    }
                    0xD4 => {
                        // SET 2,H 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xD4 (SET 2,H 2 8) not implemented");
                    }
                    0xD5 => {
                        // SET 2,L 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xD5 (SET 2,L 2 8) not implemented");
                    }
                    0xD6 => {
                        // SET 2,(HL) 2 16 | - - - -
                        unimplemented!("Prefix CB opcode 0xD6 (SET 2,(HL) 2 16) not implemented");
                    }
                    0xD7 => {
                        // SET 2,A 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xD7 (SET 2,A 2 8) not implemented");
                    }
                    0xD8 => {
                        // SET 3,B 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xD8 (SET 3,B 2 8) not implemented");
                    }
                    0xD9 => {
                        // SET 3,C 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xD9 (SET 3,C 2 8) not implemented");
                    }
                    0xDA => {
                        // SET 3,D 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xDA (SET 3,D 2 8) not implemented");
                    }
                    0xDB => {
                        // SET 3,E 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xDB (SET 3,E 2 8) not implemented");
                    }
                    0xDC => {
                        // SET 3,H 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xDC (SET 3,H 2 8) not implemented");
                    }
                    0xDD => {
                        // SET 3,L 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xDD (SET 3,L 2 8) not implemented");
                    }
                    0xDE => {
                        // SET 3,(HL) 2 16 | - - - -
                        unimplemented!("Prefix CB opcode 0xDE (SET 3,(HL) 2 16) not implemented");
                    }
                    0xDF => {
                        // SET 3,A 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xDF (SET 3,A 2 8) not implemented");
                    }
                    0xE0 => {
                        // SET 4,B 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xE0 (SET 4,B 2 8) not implemented");
                    }
                    0xE1 => {
                        // SET 4,C 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xE1 (SET 4,C 2 8) not implemented");
                    }
                    0xE2 => {
                        // SET 4,D 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xE2 (SET 4,D 2 8) not implemented");
                    }
                    0xE3 => {
                        // SET 4,E 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xE3 (SET 4,E 2 8) not implemented");
                    }
                    0xE4 => {
                        // SET 4,H 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xE4 (SET 4,H 2 8) not implemented");
                    }
                    0xE5 => {
                        // SET 4,L 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xE5 (SET 4,L 2 8) not implemented");
                    }
                    0xE6 => {
                        // SET 4,(HL) 2 16 | - - - -
                        unimplemented!("Prefix CB opcode 0xE6 (SET 4,(HL) 2 16) not implemented");
                    }
                    0xE7 => {
                        // SET 4,A 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xE7 (SET 4,A 2 8) not implemented");
                    }
                    0xE8 => {
                        // SET 5,B 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xE8 (SET 5,B 2 8) not implemented");
                    }
                    0xE9 => {
                        // SET 5,C 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xE9 (SET 5,C 2 8) not implemented");
                    }
                    0xEA => {
                        // SET 5,D 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xEA (SET 5,D 2 8) not implemented");
                    }
                    0xEB => {
                        // SET 5,E 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xEB (SET 5,E 2 8) not implemented");
                    }
                    0xEC => {
                        // SET 5,H 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xEC (SET 5,H 2 8) not implemented");
                    }
                    0xED => {
                        // SET 5,L 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xED (SET 5,L 2 8) not implemented");
                    }
                    0xEE => {
                        // SET 5,(HL) 2 16 | - - - -
                        unimplemented!("Prefix CB opcode 0xEE (SET 5,(HL) 2 16) not implemented");
                    }
                    0xEF => {
                        // SET 5,A 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xEF (SET 5,A 2 8) not implemented");
                    }
                    0xF0 => {
                        // SET 6,B 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xF0 (SET 6,B 2 8) not implemented");
                    }
                    0xF1 => {
                        // SET 6,C 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xF1 (SET 6,C 2 8) not implemented");
                    }
                    0xF2 => {
                        // SET 6,D 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xF2 (SET 6,D 2 8) not implemented");
                    }
                    0xF3 => {
                        // SET 6,E 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xF3 (SET 6,E 2 8) not implemented");
                    }
                    0xF4 => {
                        // SET 6,H 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xF4 (SET 6,H 2 8) not implemented");
                    }
                    0xF5 => {
                        // SET 6,L 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xF5 (SET 6,L 2 8) not implemented");
                    }
                    0xF6 => {
                        // SET 6,(HL) 2 16 | - - - -
                        unimplemented!("Prefix CB opcode 0xF6 (SET 6,(HL) 2 16) not implemented");
                    }
                    0xF7 => {
                        // SET 6,A 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xF7 (SET 6,A 2 8) not implemented");
                    }
                    0xF8 => {
                        // SET 7,B 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xF8 (SET 7,B 2 8) not implemented");
                    }
                    0xF9 => {
                        // SET 7,C 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xF9 (SET 7,C 2 8) not implemented");
                    }
                    0xFA => {
                        // SET 7,D 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xFA (SET 7,D 2 8) not implemented");
                    }
                    0xFB => {
                        // SET 7,E 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xFB (SET 7,E 2 8) not implemented");
                    }
                    0xFC => {
                        // SET 7,H 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xFC (SET 7,H 2 8) not implemented");
                    }
                    0xFD => {
                        // SET 7,L 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xFD (SET 7,L 2 8) not implemented");
                    }
                    0xFE => {
                        // SET 7,(HL) 2 16 | - - - -
                        unimplemented!("Prefix CB opcode 0xFE (SET 7,(HL) 2 16) not implemented");
                    }
                    0xFF => {
                        // SET 7,A 2 8 | - - - -
                        unimplemented!("Prefix CB opcode 0xFF (SET 7,A 2 8) not implemented");
                    }
                };

                self.cpu.mcycle += opcode_mcycle_prefix[op_cb as usize] as usize;
            }
            0xCC => {
                // CALL Z,a16 3 24/12 | - - - -
                unimplemented!("Opcode 0xCC (CALL Z,a16 3 24/12) not implemented");
            }
            0xCD => {
                // CALL a16 3 24 | - - - -
                unimplemented!("Opcode 0xCD (CALL a16 3 24) not implemented");
            }
            0xCE => {
                // ADC A,d8 2 8 | Z 0 H C
                let byte = self.read_op()?.wrapping_add(self.cpu.get_fc());
                self.cpu.add(byte);
            }
            0xCF => {
                // RST 08H 1 16 | - - - -
                unimplemented!("Opcode 0xCF (RST 08H 1 16) not implemented");
            }
            0xD0 => {
                // RET NC 1 20/8 | - - - -
                unimplemented!("Opcode 0xD0 (RET NC 1 20/8) not implemented");
            }
            0xD1 => {
                // POP DE 1 12 | - - - -
                self.mem.read_u16(self.cpu.sp)?;
                self.cpu.de = self.cpu.sp.wrapping_add(2);
            }
            0xD2 => {
                // JP NC,a16 3 16/12 | - - - -
                unimplemented!("Opcode 0xD2 (JP NC,a16 3 16/12) not implemented");
            }
            0xD3 => {
                // Invalid | - - - -
                unimplemented!("Opcode 0xD3 (Invalid) not implemented");
            }
            0xD4 => {
                // CALL NC,a16 3 24/12 | - - - -
                unimplemented!("Opcode 0xD4 (CALL NC,a16 3 24/12) not implemented");
            }
            0xD5 => {
                // PUSH DE 1 16 | - - - -
                self.cpu.sp = self.cpu.sp.wrapping_sub(2);
                self.mem.write_u16(self.cpu.sp, self.cpu.de)?;
            }
            0xD6 => {
                // SUB d8 2 8 | Z 1 H C
                let byte = self.read_op()?;
                self.cpu.sub(byte);
            }
            0xD7 => {
                // RST 10H 1 16 | - - - -
                unimplemented!("Opcode 0xD7 (RST 10H 1 16) not implemented");
            }
            0xD8 => {
                // RET C 1 20/8 | - - - -
                unimplemented!("Opcode 0xD8 (RET C 1 20/8) not implemented");
            }
            0xD9 => {
                // RETI 1 16 | - - - -
                unimplemented!("Opcode 0xD9 (RETI 1 16) not implemented");
            }
            0xDA => {
                // JP C,a16 3 16/12 | - - - -
                unimplemented!("Opcode 0xDA (JP C,a16 3 16/12) not implemented");
            }
            0xDB => {
                // Invalid | - - - -
                unimplemented!("Opcode 0xDB (Invalid) not implemented");
            }
            0xDC => {
                // CALL C,a16 3 24/12 | - - - -
                unimplemented!("Opcode 0xDC (CALL C,a16 3 24/12) not implemented");
            }
            0xDD => {
                // Invalid | - - - -
                unimplemented!("Opcode 0xDD (Invalid) not implemented");
            }
            0xDE => {
                // SBC A,d8 2 8 | Z 1 H C
                let byte = self.read_op()?;
                self.cpu.sub(byte);
            }
            0xDF => {
                // RST 18H 1 16 | - - - -
                unimplemented!("Opcode 0xDF (RST 18H 1 16) not implemented");
            }
            0xE0 => {
                // LDH (a8),A 2 12 | - - - -
                let byte = self.cpu.get_a();
                let word = 0xFF00u16 | self.read_op()? as u16;
                self.mem.write(word, byte)?;
            }
            0xE1 => {
                // POP HL 1 12 | - - - -
                self.mem.read_u16(self.cpu.sp)?;
                self.cpu.hl = self.cpu.sp.wrapping_add(2);
            }
            0xE2 => {
                // LD (C),A 2 8 | - - - -
                let byte = self.cpu.get_a();
                let word = 0xFF00u16 | self.cpu.get_c() as u16;
                self.mem.write(word, byte)?;
            }
            0xE3 => {
                // Invalid | - - - -
                unimplemented!("Opcode 0xE3 (Invalid) not implemented");
            }
            0xE4 => {
                // Invalid | - - - -
                unimplemented!("Opcode 0xE4 (Invalid) not implemented");
            }
            0xE5 => {
                // PUSH HL 1 16 | - - - -
                self.cpu.sp = self.cpu.sp.wrapping_sub(2);
                self.mem.write_u16(self.cpu.sp, self.cpu.hl)?;
            }
            0xE6 => {
                // AND d8 2 8 | Z 0 1 0
                let byte = self.read_op()?;
                self.cpu.and(byte);
            }
            0xE7 => {
                // RST 20H 1 16 | - - - -
                unimplemented!("Opcode 0xE7 (RST 20H 1 16) not implemented");
            }
            0xE8 => {
                // ADD SP,r8 2 16 | 0 0 H C
                unimplemented!("Opcode 0xE8 (ADD SP,r8 2 16) not implemented");
            }
            0xE9 => {
                // JP (HL) 1 4 | - - - -
                unimplemented!("Opcode 0xE9 (JP (HL) 1 4) not implemented");
            }
            0xEA => {
                // LD (a16),A 3 16 | - - - -
                let word = self.read_op_imm16()?;
                let byte = self.cpu.get_a();
                self.mem.write(word, byte)?;
            }
            0xEB => {
                // Invalid | - - - -
                unimplemented!("Opcode 0xEB (Invalid) not implemented");
            }
            0xEC => {
                // Invalid | - - - -
                unimplemented!("Opcode 0xEC (Invalid) not implemented");
            }
            0xED => {
                // Invalid | - - - -
                unimplemented!("Opcode 0xED (Invalid) not implemented");
            }
            0xEE => {
                // XOR d8 2 8 | Z 0 0 0
                let byte = self.read_op()?;
                self.cpu.xor(byte);
            }
            0xEF => {
                // RST 28H 1 16 | - - - -
                unimplemented!("Opcode 0xEF (RST 28H 1 16) not implemented");
            }
            0xF0 => {
                // LDH A,(a8) 2 12 | - - - -
                let word = 0xFF00u16 | self.read_op()? as u16;
                let byte = self.mem.read(word)?;
                self.cpu.set_a(byte);
            }
            0xF1 => {
                // POP AF 1 12 | Z N H C
                self.mem.read_u16(self.cpu.sp)?;
                self.cpu.af = self.cpu.sp.wrapping_add(2);
            }
            0xF2 => {
                // LD A,(C) 2 8 | - - - -
                let word = 0xFF00u16 | self.cpu.get_c() as u16;
                let byte = self.mem.read(word)?;
                self.cpu.set_a(byte);
            }
            0xF3 => {
                // DI 1 4 | - - - -
                unimplemented!("Opcode 0xF3 (DI 1 4) not implemented");
            }
            0xF4 => {
                // Invalid | - - - -
                unimplemented!("Opcode 0xF4 (Invalid) not implemented");
            }
            0xF5 => {
                // PUSH AF 1 16 | - - - -
                self.cpu.sp = self.cpu.sp.wrapping_sub(2);
                self.mem.write_u16(self.cpu.sp, self.cpu.af)?;
            }
            0xF6 => {
                // OR d8 2 8 | Z 0 0 0
                let byte = self.read_op()?;
                self.cpu.or(byte);
            }
            0xF7 => {
                // RST 30H 1 16 | - - - -
                unimplemented!("Opcode 0xF7 (RST 30H 1 16) not implemented");
            }
            0xF8 => {
                // LD HL,SP+r8 2 12 | 0 0 H C
                let word = self.read_op()? as u16;
                let is_carry = is_carry_add_u16(self.cpu.sp, word);
                let is_half_carry = is_half_carry_add_u16(self.cpu.sp, word);
                self.cpu.hl = self.cpu.sp.wrapping_add(word);

                self.cpu.set_fc(is_carry);
                self.cpu.set_fh(is_half_carry);
                self.cpu.set_fz(false);
                self.cpu.set_fn(false);
            }
            0xF9 => {
                // LD SP,HL 1 8 | - - - -
                self.cpu.sp = self.cpu.hl;
            }
            0xFA => {
                // LD A,(a16) 3 16 | - - - -
                let word = self.read_op_imm16()?;
                let byte = self.mem.read(word)?;
                self.cpu.set_a(byte);
            }
            0xFB => {
                // EI 1 4 | - - - -
                unimplemented!("Opcode 0xFB (EI 1 4) not implemented");
            }
            0xFC => {
                // Invalid | - - - -
                unimplemented!("Opcode 0xFC (Invalid) not implemented");
            }
            0xFD => {
                // Invalid | - - - -
                unimplemented!("Opcode 0xFD (Invalid) not implemented");
            }
            0xFE => {
                // CP d8 2 8 | Z 1 H C
                let byte = self.read_op()?;
                self.cpu.cp(byte);
            }
            0xFF => {
                // RST 38H 1 16 | - - - -
                unimplemented!("Opcode 0xFF (RST 38H 1 16) not implemented");
            }
        };

        self.cpu.mcycle += opcode_mcycle[op as usize] as usize;

        Ok(())
    }

    fn read_op(&mut self) -> Result<u8, Error> {
        let op = self.mem.read(self.cpu.pc)?;
        self.cpu.pc = self.cpu.pc.wrapping_add(1);

        Ok(op)
    }

    fn read_op_imm16(&mut self) -> Result<u16, Error> {
        let lo = self.read_op()?;
        let hi = self.read_op()?;

        Ok(((hi as u16) << 8) | lo as u16)
    }
}
