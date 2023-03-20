use std::fs::File;
use std::io::stdin;
use std::io::stdout;
use std::io::Read;
use std::io::Write;

use crate::cartridge::Cartridge;
use crate::conf::*;
use crate::cpu::*;
use crate::debugger::DebugCmd;
use crate::debugger::Debugger;
use crate::mem::*;
use crate::util::*;

enum State {
    Running,
    Stop,
}

pub struct VM {
    mem: Mem,
    cpu: Cpu,
    debugger: Debugger,
    counter: u64,
    state: State,
    div_ticker: u16,
    tima_ticker: u32,
}

impl VM {
    pub fn new(cartridge: Cartridge, debugger: Debugger) -> Result<Self, Error> {
        Ok(VM {
            mem: Mem::new(cartridge)?,
            cpu: Cpu::new(),
            debugger,
            counter: 0,
            state: State::Running,
            div_ticker: 0,
            tima_ticker: 0,
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
        self.reset()?;

        log::info!("VM eval loop start");

        loop {
            if self.debugger.should_stop(self.cpu.pc) {
                loop {
                    match self.read_repl()? {
                        Some(DebugCmd::Quit) => return Ok(()),
                        Some(DebugCmd::Next(auto_step)) => {
                            self.debugger.set_auto_step_count(auto_step - 1);
                            break;
                        }
                        Some(DebugCmd::Print) => self.print_debug_panel(),
                        Some(DebugCmd::Continue) => {
                            self.debugger.clear_steps();
                            break;
                        }
                        None => (),
                    };
                }
            }

            if let State::Running = self.state {
                let old_tac = self.mem.read_unchecked(MEM_LOC_TAC)?;

                self.exec_op()?;
                self.handle_ticks(old_tac)?;
                self.counter += 1;
            }
        }
    }

    fn reset(&mut self) -> Result<(), Error> {
        self.mem.reset()?;

        log::info!("VM reset");

        Ok(())
    }

    fn exec_op(&mut self) -> Result<(), Error> {
        let mut is_alternative_mcycle = false;
        let op = self.read_op()?;
        log::debug!(
            "AF={:#06X} BC={:#06X} DE={:#06X} HL={:#06X} SP={:#06X} PC={:#06X} | {:#4X?}: {}",
            self.cpu.af,
            self.cpu.bc,
            self.cpu.de,
            self.cpu.hl,
            self.cpu.sp,
            self.cpu.pc - 1,
            op,
            OPCODE_NAME[op as usize]
        );

        match op {
            0x00 => {
                // NOP 1 4 | - - - -
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
                let is_carry = is_carry_rot_left_u8(self.cpu.get_a(), 1);
                let new_a = self.cpu.get_a().rotate_left(1);
                self.cpu.set_a(new_a);
                self.cpu.set_fz(false);
                self.cpu.set_fn(false);
                self.cpu.set_fh(false);
                self.cpu.set_fc(is_carry);
            }
            0x08 => {
                // LD (a16),SP 3 20 | - - - -
                let word = self.read_op_imm16()?;
                self.mem.write_u16(word, self.cpu.sp)?;
            }
            0x09 => {
                // ADD HL,BC 1 8 | - 0 H C
                let is_half_carry = is_half_carry_add_u16(self.cpu.hl, self.cpu.bc);
                let is_carry = is_carry_add_u16(self.cpu.hl, self.cpu.bc);

                self.cpu.hl = self.cpu.hl.wrapping_add(self.cpu.bc);

                self.cpu.set_fn(false);
                self.cpu.set_fh(is_half_carry);
                self.cpu.set_fc(is_carry);
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
                let is_carry = is_carry_rot_right_u8(self.cpu.get_a(), 1);
                let new_a = self.cpu.get_a().rotate_right(1);
                self.cpu.set_a(new_a);
                self.cpu.set_fz(false);
                self.cpu.set_fn(false);
                self.cpu.set_fh(false);
                self.cpu.set_fc(is_carry);
            }
            0x10 => {
                // STOP 0 2 4 | - - - -
                self.state = State::Stop;
                self.mem.write(MEM_LOC_DIV, 0)?;
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
                let old_carry = self.cpu.get_fc();
                let is_carry = is_carry_rot_left_u8(self.cpu.get_a(), 1);
                let new_a = self.cpu.get_a().rotate_left(1) | old_carry;

                self.cpu.set_a(new_a);

                self.cpu.set_fz(false);
                self.cpu.set_fn(false);
                self.cpu.set_fh(false);
                self.cpu.set_fc(is_carry);
            }
            0x18 => {
                // JR r8 2 12 | - - - -
                let offs = self.read_op()? as i8;
                self.cpu.pc = wrapping_add_u16_i8(self.cpu.pc, offs);
            }
            0x19 => {
                // ADD HL,DE 1 8 | - 0 H C
                let is_half_carry = is_half_carry_add_u16(self.cpu.hl, self.cpu.de);
                let is_carry = is_carry_add_u16(self.cpu.hl, self.cpu.de);

                self.cpu.hl = self.cpu.hl.wrapping_add(self.cpu.de);

                self.cpu.set_fn(false);
                self.cpu.set_fh(is_half_carry);
                self.cpu.set_fc(is_carry);
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
                let old_carry = self.cpu.get_fc();
                let is_carry = is_carry_rot_right_u8(self.cpu.get_a(), 1);
                let new_a = self.cpu.get_a().rotate_right(1) | (old_carry << 7);

                self.cpu.set_a(new_a);

                self.cpu.set_fz(false);
                self.cpu.set_fn(false);
                self.cpu.set_fh(false);
                self.cpu.set_fc(is_carry);
            }
            0x20 => {
                // JR NZ,r8 2 12/8 | - - - -
                let offs = self.read_op()? as i8;
                let new_pc = wrapping_add_u16_i8(self.cpu.pc, offs);
                if !self.cpu.is_fz() {
                    self.cpu.pc = new_pc;
                } else {
                    is_alternative_mcycle = true;
                }
            }
            0x21 => {
                // LD HL,d16 3 12 | - - - -
                let word = self.read_op_imm16()?;
                self.cpu.hl = word;
            }
            0x22 => {
                // LD (HL+),A 1 8 | - - - -
                let byte = self.cpu.get_a();
                self.write_hl(byte)?;
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
                if !self.cpu.is_fn() {
                    // It was addition before.
                    if self.cpu.is_fc() || self.cpu.get_a() > 0x99 {
                        let a = self.cpu.get_a();
                        self.cpu.set_a(a.wrapping_add(0x60));
                        self.cpu.set_fc(true);
                    }

                    if self.cpu.is_fh() || (self.cpu.get_a() & 0xf) > 0x9 {
                        let a = self.cpu.get_a();
                        self.cpu.set_a(a.wrapping_add(0x6));
                    }
                } else {
                    // It was substraction before.
                    if self.cpu.is_fc() {
                        let a = self.cpu.get_a();
                        self.cpu.set_a(a.wrapping_sub(0x60));
                    }

                    if self.cpu.is_fh() {
                        let a = self.cpu.get_a();
                        self.cpu.set_a(a.wrapping_sub(0x6));
                    }
                }

                let a = self.cpu.get_a();
                self.cpu.set_fz(a == 0);
                self.cpu.set_fh(false);
            }
            0x28 => {
                // JR Z,r8 2 12/8 | - - - -
                let offs = self.read_op()? as i8;
                let new_pc = wrapping_add_u16_i8(self.cpu.pc, offs);
                if self.cpu.is_fz() {
                    self.cpu.pc = new_pc;
                } else {
                    is_alternative_mcycle = true;
                }
            }
            0x29 => {
                // ADD HL,HL 1 8 | - 0 H C
                let is_half_carry = is_half_carry_add_u16(self.cpu.hl, self.cpu.hl);
                let is_carry = is_carry_add_u16(self.cpu.hl, self.cpu.hl);

                self.cpu.hl = self.cpu.hl.wrapping_add(self.cpu.hl);

                self.cpu.set_fn(false);
                self.cpu.set_fh(is_half_carry);
                self.cpu.set_fc(is_carry);
            }
            0x2A => {
                // LD A,(HL+) 1 8 | - - - -
                let byte = self.read_hl()?;
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
                let a = self.cpu.get_a();
                self.cpu.set_a(!a);
                self.cpu.set_fn(true);
                self.cpu.set_fh(true);
            }
            0x30 => {
                // JR NC,r8 2 12/8 | - - - -
                let offs = self.read_op()? as i8;
                let new_pc = wrapping_add_u16_i8(self.cpu.pc, offs);
                if !self.cpu.is_fc() {
                    self.cpu.pc = new_pc;
                } else {
                    is_alternative_mcycle = true;
                }
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
                let byte = self.read_hl()?;
                let is_half_carry = is_half_carry_add_u8(byte, 1);

                self.write_hl(byte.wrapping_add(1))?;
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(false);
                self.cpu.set_fh(is_half_carry);
            }
            0x35 => {
                // DEC (HL) 1 12 | Z 1 H -
                let byte = self.read_hl()?;
                let is_half_carry = is_half_carry_sub_u8(byte, 1);

                self.write_hl(byte.wrapping_sub(1))?;
                self.cpu.set_fz(byte == 0);
                self.cpu.set_fn(true);
                self.cpu.set_fh(is_half_carry);
            }
            0x36 => {
                // LD (HL),d8 2 12 | - - - -
                let byte = self.read_op()?;
                self.write_hl(byte)?;
            }
            0x37 => {
                // SCF 1 4 | - 0 0 1
                self.cpu.set_fn(false);
                self.cpu.set_fh(false);
                self.cpu.set_fc(true);
            }
            0x38 => {
                // JR C,r8 2 12/8 | - - - -
                let offs = self.read_op()? as i8;
                let new_pc = wrapping_add_u16_i8(self.cpu.pc, offs);
                if self.cpu.is_fc() {
                    self.cpu.pc = new_pc;
                } else {
                    is_alternative_mcycle = true;
                }
            }
            0x39 => {
                // ADD HL,SP 1 8 | - 0 H C
                let is_carry = is_carry_add_u16(self.cpu.hl, self.cpu.sp);
                let is_half_carry = is_half_carry_add_u16(self.cpu.hl, self.cpu.sp);
                self.cpu.hl = self.cpu.hl.wrapping_add(self.cpu.sp);
                self.cpu.set_fn(false);
                self.cpu.set_fh(is_half_carry);
                self.cpu.set_fc(is_carry);
            }
            0x3A => {
                // LD A,(HL-) 1 8 | - - - -
                let byte = self.read_hl()?;
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
                self.cpu.set_fn(false);
                self.cpu.set_fh(false);
                let is_c = self.cpu.get_fc() > 0;
                self.cpu.set_fc(!is_c);
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
                let byte = self.read_hl()?;
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
                let byte = self.read_hl()?;
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
                let byte = self.read_hl()?;
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
                let byte = self.read_hl()?;
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
                let byte = self.read_hl()?;
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
                let byte = self.read_hl()?;
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
                self.write_hl(byte)?;
            }
            0x71 => {
                // LD (HL),C 1 8 | - - - -
                let byte = self.cpu.get_c();
                self.write_hl(byte)?;
            }
            0x72 => {
                // LD (HL),D 1 8 | - - - -
                let byte = self.cpu.get_d();
                self.write_hl(byte)?;
            }
            0x73 => {
                // LD (HL),E 1 8 | - - - -
                let byte = self.cpu.get_e();
                self.write_hl(byte)?;
            }
            0x74 => {
                // LD (HL),H 1 8 | - - - -
                let byte = self.cpu.get_h();
                self.write_hl(byte)?;
            }
            0x75 => {
                // LD (HL),L 1 8 | - - - -
                let byte = self.cpu.get_l();
                self.write_hl(byte)?;
            }
            0x76 => {
                // HALT 1 4 | - - - -
                unimplemented!("Opcode 0x76 (HALT 1 4) not implemented");
            }
            0x77 => {
                // LD (HL),A 1 8 | - - - -
                let byte = self.cpu.get_a();
                self.write_hl(byte)?;
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
                let byte = self.read_hl()?;
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
                let byte = self.read_hl()?;
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
                let byte = self.read_hl()?;
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
                let byte = self.read_hl()?;
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
                let byte = self.read_hl()?;
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
                let byte = self.read_hl()?;
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
                let byte = self.read_hl()?;
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
                let byte = self.read_hl()?;
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
                let byte = self.read_hl()?;
                self.cpu.cp(byte);
            }
            0xBF => {
                // CP A 1 4 | Z 1 H C
                let byte = self.cpu.get_a();
                self.cpu.cp(byte);
            }
            0xC0 => {
                // RET NZ 1 20/8 | - - - -
                if !self.cpu.is_fz() {
                    let addr = self.pop_u16()?;
                    self.cpu.pc = addr;
                } else {
                    is_alternative_mcycle = true;
                }
            }
            0xC1 => {
                // POP BC 1 12 | - - - -
                self.cpu.bc = self.pop_u16()?;
            }
            0xC2 => {
                // JP NZ,a16 3 16/12 | - - - -
                let addr = self.read_op_imm16()?;
                if !self.cpu.is_fz() {
                    self.cpu.pc = addr;
                } else {
                    is_alternative_mcycle = true;
                }
            }
            0xC3 => {
                // JP a16 3 16 | - - - -
                let addr = self.read_op_imm16()?;
                self.cpu.pc = addr;
            }
            0xC4 => {
                // CALL NZ,a16 3 24/12 | - - - -
                let addr = self.read_op_imm16()?;

                if !self.cpu.is_fz() {
                    self.push_u16(self.cpu.pc)?;
                    self.cpu.pc = addr;
                } else {
                    is_alternative_mcycle = true;
                }
            }
            0xC5 => {
                // PUSH BC 1 16 | - - - -
                self.push_u16(self.cpu.bc)?;
            }
            0xC6 => {
                // ADD A,d8 2 8 | Z 0 H C
                let byte = self.read_op()?;
                self.cpu.add(byte);
            }
            0xC7 => {
                // RST 00H 1 16 | - - - -
                self.push_u16(self.cpu.pc)?;
                self.cpu.pc = (op - 0xC7) as u16;
            }
            0xC8 => {
                // RET Z 1 20/8 | - - - -
                if self.cpu.is_fz() {
                    let addr = self.pop_u16()?;
                    self.cpu.pc = addr;
                } else {
                    is_alternative_mcycle = true;
                }
            }
            0xC9 => {
                // RET 1 16 | - - - -
                let addr = self.pop_u16()?;
                self.cpu.pc = addr;
            }
            0xCA => {
                // JP Z,a16 3 16/12 | - - - -
                let addr = self.read_op_imm16()?;
                if self.cpu.is_fz() {
                    self.cpu.pc = addr;
                } else {
                    is_alternative_mcycle = true;
                }
            }
            0xCB => {
                // PREFIX CB 1 4 | - - - -
                let op_cb = self.read_op()?;

                log::debug!(
                    "AF={:#06X} BC={:#06X} DE={:#06X} HL={:#06X} SP={:#06X} PC={:#06X} | {:#4X?}: {}",
                    self.cpu.af,
                    self.cpu.bc,
                    self.cpu.de,
                    self.cpu.hl,
                    self.cpu.sp,
                    self.cpu.pc - 1,
                    op_cb,
                    OPCODE_CB_NAME[op_cb as usize]
                );

                match op_cb {
                    0x00 => {
                        // RLC B 2 8F | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_b(), 1);
                        let new_b = self.cpu.get_b().rotate_left(1);

                        self.cpu.set_b(new_b);
                        self.cpu.set_fz(new_b == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x01 => {
                        // RLC C 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_c(), 1);
                        let new_c = self.cpu.get_c().rotate_left(1);

                        self.cpu.set_c(new_c);
                        self.cpu.set_fz(new_c == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x02 => {
                        // RLC D 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_d(), 1);
                        let new_d = self.cpu.get_d().rotate_left(1);

                        self.cpu.set_d(new_d);
                        self.cpu.set_fz(new_d == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x03 => {
                        // RLC E 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_e(), 1);
                        let new_e = self.cpu.get_e().rotate_left(1);

                        self.cpu.set_e(new_e);
                        self.cpu.set_fz(new_e == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x04 => {
                        // RLC H 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_h(), 1);
                        let new_h = self.cpu.get_h().rotate_left(1);

                        self.cpu.set_h(new_h);
                        self.cpu.set_fz(new_h == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x05 => {
                        // RLC L 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_l(), 1);
                        let new_l = self.cpu.get_l().rotate_left(1);

                        self.cpu.set_l(new_l);
                        self.cpu.set_fz(new_l == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x06 => {
                        // RLC (HL) 2 16 | Z 0 0 C
                        let byte = self.read_hl()?;

                        let is_carry = is_carry_rot_left_u8(byte, 1);
                        let new_byte = byte.rotate_left(1);

                        self.write_hl(new_byte)?;

                        self.cpu.set_fz(new_byte == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x07 => {
                        // RLC A 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_a(), 1);
                        let new_a = self.cpu.get_a().rotate_left(1);

                        self.cpu.set_a(new_a);
                        self.cpu.set_fz(new_a == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x08 => {
                        // RRC B 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_b(), 1);
                        let new_b = self.cpu.get_b().rotate_right(1);

                        self.cpu.set_b(new_b);
                        self.cpu.set_fz(new_b == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x09 => {
                        // RRC C 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_c(), 1);
                        let new_c = self.cpu.get_c().rotate_right(1);

                        self.cpu.set_c(new_c);
                        self.cpu.set_fz(new_c == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x0A => {
                        // RRC D 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_d(), 1);
                        let new_d = self.cpu.get_d().rotate_right(1);

                        self.cpu.set_d(new_d);
                        self.cpu.set_fz(new_d == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x0B => {
                        // RRC E 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_e(), 1);
                        let new_e = self.cpu.get_e().rotate_right(1);

                        self.cpu.set_e(new_e);
                        self.cpu.set_fz(new_e == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x0C => {
                        // RRC H 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_h(), 1);
                        let new_h = self.cpu.get_h().rotate_right(1);

                        self.cpu.set_h(new_h);
                        self.cpu.set_fz(new_h == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x0D => {
                        // RRC L 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_l(), 1);
                        let new_l = self.cpu.get_l().rotate_right(1);

                        self.cpu.set_l(new_l);
                        self.cpu.set_fz(new_l == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x0E => {
                        // RRC (HL) 2 16 | Z 0 0 C
                        let byte = self.read_hl()?;

                        let is_carry = is_carry_rot_right_u8(byte, 1);
                        let new_byte = byte.rotate_right(1);

                        self.write_hl(new_byte)?;

                        self.cpu.set_fz(new_byte == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x0F => {
                        // RRC A 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_a(), 1);
                        let new_a = self.cpu.get_a().rotate_right(1);

                        self.cpu.set_a(new_a);
                        self.cpu.set_fz(new_a == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x10 => {
                        // RL B 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_b(), 1);
                        let byte = self.cpu.get_b().rotate_left(1);

                        self.cpu.set_b(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x11 => {
                        // RL C 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_c(), 1);
                        let byte = self.cpu.get_c().rotate_left(1);

                        self.cpu.set_c(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x12 => {
                        // RL D 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_d(), 1);
                        let byte = self.cpu.get_d().rotate_left(1);

                        self.cpu.set_d(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x13 => {
                        // RL E 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_e(), 1);
                        let byte = self.cpu.get_e().rotate_left(1);

                        self.cpu.set_e(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x14 => {
                        // RL H 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_h(), 1);
                        let byte = self.cpu.get_h().rotate_left(1);

                        self.cpu.set_h(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x15 => {
                        // RL L 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_l(), 1);
                        let byte = self.cpu.get_l().rotate_left(1);

                        self.cpu.set_l(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x16 => {
                        // RL (HL) 2 16 | Z 0 0 C
                        let byte = self.read_hl()?;
                        let is_carry = is_carry_rot_left_u8(byte, 1);
                        let new_byte = byte.rotate_left(1);

                        self.write_hl(new_byte)?;
                        self.cpu.set_flags(new_byte == 0, false, false, is_carry);
                    }
                    0x17 => {
                        // RL A 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_a(), 1);
                        let byte = self.cpu.get_a().rotate_left(1);

                        self.cpu.set_a(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x18 => {
                        // RR B 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_b(), 1);
                        let byte = self.cpu.get_b().rotate_right(1);

                        self.cpu.set_b(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x19 => {
                        // RR C 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_c(), 1);
                        let byte = self.cpu.get_c().rotate_right(1);

                        self.cpu.set_c(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x1A => {
                        // RR D 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_d(), 1);
                        let byte = self.cpu.get_d().rotate_right(1);

                        self.cpu.set_d(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x1B => {
                        // RR E 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_e(), 1);
                        let byte = self.cpu.get_e().rotate_right(1);

                        self.cpu.set_e(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x1C => {
                        // RR H 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_h(), 1);
                        let byte = self.cpu.get_h().rotate_right(1);

                        self.cpu.set_h(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x1D => {
                        // RR L 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_l(), 1);
                        let byte = self.cpu.get_l().rotate_right(1);

                        self.cpu.set_l(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x1E => {
                        // RR (HL) 2 16 | Z 0 0 C
                        let byte = self.read_hl()?;
                        let is_carry = is_carry_rot_right_u8(byte, 1);
                        let new_byte = byte.rotate_right(1);

                        self.write_hl(new_byte)?;
                        self.cpu.set_flags(new_byte == 0, false, false, is_carry);
                    }
                    0x1F => {
                        // RR A 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_a(), 1);
                        let byte = self.cpu.get_a().rotate_right(1);

                        self.cpu.set_a(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x20 => {
                        // SLA B 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_left_u8(self.cpu.get_b(), 1);
                        let byte = shift_left_a(self.cpu.get_b());

                        self.cpu.set_b(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x21 => {
                        // SLA C 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_left_u8(self.cpu.get_c(), 1);
                        let byte = shift_left_a(self.cpu.get_c());

                        self.cpu.set_c(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x22 => {
                        // SLA D 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_left_u8(self.cpu.get_d(), 1);
                        let byte = shift_left_a(self.cpu.get_d());

                        self.cpu.set_d(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x23 => {
                        // SLA E 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_left_u8(self.cpu.get_e(), 1);
                        let byte = shift_left_a(self.cpu.get_e());

                        self.cpu.set_e(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x24 => {
                        // SLA H 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_left_u8(self.cpu.get_h(), 1);
                        let byte = shift_left_a(self.cpu.get_h());

                        self.cpu.set_h(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x25 => {
                        // SLA L 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_left_u8(self.cpu.get_l(), 1);
                        let byte = shift_left_a(self.cpu.get_l());

                        self.cpu.set_l(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x26 => {
                        // SLA (HL) 2 16 | Z 0 0 C
                        let byte = self.read_hl()?;
                        let is_carry = is_carry_shift_left_u8(self.cpu.get_a(), 1);
                        let new_byte = shift_left_a(byte);

                        self.write_hl(new_byte)?;
                        self.cpu.set_flags(new_byte == 0, false, false, is_carry);
                    }
                    0x27 => {
                        // SLA A 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_left_u8(self.cpu.get_a(), 1);
                        let byte = shift_left_a(self.cpu.get_a());

                        self.cpu.set_a(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x28 => {
                        // SRA B 2 8 | Z 0 0 0
                        let byte = shift_right_a(self.cpu.get_b());

                        self.cpu.set_b(byte);
                        self.cpu.set_flags(byte == 0, false, false, false);
                    }
                    0x29 => {
                        // SRA C 2 8 | Z 0 0 0
                        let byte = shift_right_a(self.cpu.get_c());

                        self.cpu.set_c(byte);
                        self.cpu.set_flags(byte == 0, false, false, false);
                    }
                    0x2A => {
                        // SRA D 2 8 | Z 0 0 0
                        let byte = shift_right_a(self.cpu.get_d());

                        self.cpu.set_d(byte);
                        self.cpu.set_flags(byte == 0, false, false, false);
                    }
                    0x2B => {
                        // SRA E 2 8 | Z 0 0 0
                        let byte = shift_right_a(self.cpu.get_e());

                        self.cpu.set_e(byte);
                        self.cpu.set_flags(byte == 0, false, false, false);
                    }
                    0x2C => {
                        // SRA H 2 8 | Z 0 0 0
                        let byte = shift_right_a(self.cpu.get_h());

                        self.cpu.set_h(byte);
                        self.cpu.set_flags(byte == 0, false, false, false);
                    }
                    0x2D => {
                        // SRA L 2 8 | Z 0 0 0
                        let byte = shift_right_a(self.cpu.get_l());

                        self.cpu.set_l(byte);
                        self.cpu.set_flags(byte == 0, false, false, false);
                    }
                    0x2E => {
                        // SRA (HL) 2 16 | Z 0 0 0
                        let byte = shift_left_a(self.read_hl()?);

                        self.write_hl(byte)?;
                        self.cpu.set_flags(byte == 0, false, false, false);
                    }
                    0x2F => {
                        // SRA A 2 8 | Z 0 0 0
                        let byte = shift_right_a(self.cpu.get_a());

                        self.cpu.set_a(byte);
                        self.cpu.set_flags(byte == 0, false, false, false);
                    }
                    0x30 => {
                        // SWAP B 2 8 | Z 0 0 0
                        let byte = swap(self.cpu.get_b());
                        self.cpu.set_b(byte);
                        self.cpu.set_flags(byte == 0, false, false, false);
                    }
                    0x31 => {
                        // SWAP C 2 8 | Z 0 0 0
                        let byte = swap(self.cpu.get_c());
                        self.cpu.set_c(byte);
                        self.cpu.set_flags(byte == 0, false, false, false);
                    }
                    0x32 => {
                        // SWAP D 2 8 | Z 0 0 0
                        let byte = swap(self.cpu.get_d());
                        self.cpu.set_d(byte);
                        self.cpu.set_flags(byte == 0, false, false, false);
                    }
                    0x33 => {
                        // SWAP E 2 8 | Z 0 0 0
                        let byte = swap(self.cpu.get_e());
                        self.cpu.set_e(byte);
                        self.cpu.set_flags(byte == 0, false, false, false);
                    }
                    0x34 => {
                        // SWAP H 2 8 | Z 0 0 0
                        let byte = swap(self.cpu.get_h());
                        self.cpu.set_h(byte);
                        self.cpu.set_flags(byte == 0, false, false, false);
                    }
                    0x35 => {
                        // SWAP L 2 8 | Z 0 0 0
                        let byte = swap(self.cpu.get_l());
                        self.cpu.set_l(byte);
                        self.cpu.set_flags(byte == 0, false, false, false);
                    }
                    0x36 => {
                        // SWAP (HL) 2 16 | Z 0 0 0
                        let byte = swap(self.read_hl()?);

                        self.write_hl(byte)?;
                        self.cpu.set_flags(byte == 0, false, false, false);
                    }
                    0x37 => {
                        // SWAP A 2 8 | Z 0 0 0
                        let byte = swap(self.cpu.get_a());
                        self.cpu.set_a(byte);
                        self.cpu.set_flags(byte == 0, false, false, false);
                    }
                    0x38 => {
                        // SRL B 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_right_u8(self.cpu.get_b(), 1);
                        let byte = shift_right_l(self.cpu.get_b());

                        self.cpu.set_b(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x39 => {
                        // SRL C 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_right_u8(self.cpu.get_c(), 1);
                        let byte = shift_right_l(self.cpu.get_c());

                        self.cpu.set_c(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x3A => {
                        // SRL D 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_right_u8(self.cpu.get_d(), 1);
                        let byte = shift_right_l(self.cpu.get_d());

                        self.cpu.set_d(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x3B => {
                        // SRL E 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_right_u8(self.cpu.get_e(), 1);
                        let byte = shift_right_l(self.cpu.get_e());

                        self.cpu.set_e(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x3C => {
                        // SRL H 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_right_u8(self.cpu.get_h(), 1);
                        let byte = shift_right_l(self.cpu.get_h());

                        self.cpu.set_h(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x3D => {
                        // SRL L 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_right_u8(self.cpu.get_l(), 1);
                        let byte = shift_right_l(self.cpu.get_l());

                        self.cpu.set_l(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x3E => {
                        // SRL (HL) 2 16 | Z 0 0 C
                        let byte = self.read_hl()?;
                        let is_carry = is_carry_shift_right_u8(byte, 1);
                        let new_byte = shift_right_l(byte);

                        self.write_hl(new_byte)?;
                        self.cpu.set_flags(new_byte == 0, false, false, is_carry);
                    }
                    0x3F => {
                        // SRL A 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_right_u8(self.cpu.get_a(), 1);
                        let byte = shift_right_l(self.cpu.get_a());

                        self.cpu.set_a(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x40 => {
                        // BIT 0,B 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_b(), 0);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x41 => {
                        // BIT 0,C 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_c(), 0);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x42 => {
                        // BIT 0,D 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_d(), 0);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x43 => {
                        // BIT 0,E 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_e(), 0);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x44 => {
                        // BIT 0,H 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_h(), 0);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x45 => {
                        // BIT 0,L 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_l(), 0);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x46 => {
                        // BIT 0,(HL) 2 16 | Z 0 1 -
                        let is_bit = is_bit(self.read_hl()?, 0);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x47 => {
                        // BIT 0,A 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_a(), 0);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x48 => {
                        // BIT 1,B 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_b(), 1);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x49 => {
                        // BIT 1,C 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_c(), 1);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x4A => {
                        // BIT 1,D 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_d(), 1);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x4B => {
                        // BIT 1,E 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_e(), 1);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x4C => {
                        // BIT 1,H 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_h(), 1);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x4D => {
                        // BIT 1,L 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_l(), 1);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x4E => {
                        // BIT 1,(HL) 2 16 | Z 0 1 -
                        let is_bit = is_bit(self.read_hl()?, 1);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x4F => {
                        // BIT 1,A 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_a(), 1);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x50 => {
                        // BIT 2,B 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_b(), 2);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x51 => {
                        // BIT 2,C 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_c(), 2);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x52 => {
                        // BIT 2,D 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_d(), 2);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x53 => {
                        // BIT 2,E 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_e(), 2);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x54 => {
                        // BIT 2,H 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_h(), 2);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x55 => {
                        // BIT 2,L 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_l(), 2);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x56 => {
                        // BIT 2,(HL) 2 16 | Z 0 1 -
                        let is_bit = is_bit(self.read_hl()?, 2);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x57 => {
                        // BIT 2,A 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_a(), 2);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x58 => {
                        // BIT 3,B 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_b(), 3);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x59 => {
                        // BIT 3,C 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_c(), 3);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x5A => {
                        // BIT 3,D 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_d(), 3);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x5B => {
                        // BIT 3,E 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_e(), 3);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x5C => {
                        // BIT 3,H 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_h(), 3);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x5D => {
                        // BIT 3,L 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_l(), 3);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x5E => {
                        // BIT 3,(HL) 2 16 | Z 0 1 -
                        let is_bit = is_bit(self.read_hl()?, 3);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x5F => {
                        // BIT 3,A 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_a(), 3);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x60 => {
                        // BIT 4,B 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_b(), 4);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x61 => {
                        // BIT 4,C 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_c(), 4);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x62 => {
                        // BIT 4,D 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_d(), 4);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x63 => {
                        // BIT 4,E 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_e(), 4);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x64 => {
                        // BIT 4,H 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_h(), 4);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x65 => {
                        // BIT 4,L 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_l(), 4);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x66 => {
                        // BIT 4,(HL) 2 16 | Z 0 1 -
                        let is_bit = is_bit(self.read_hl()?, 4);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x67 => {
                        // BIT 4,A 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_a(), 4);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x68 => {
                        // BIT 5,B 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_b(), 5);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x69 => {
                        // BIT 5,C 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_c(), 5);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x6A => {
                        // BIT 5,D 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_d(), 5);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x6B => {
                        // BIT 5,E 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_e(), 5);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x6C => {
                        // BIT 5,H 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_h(), 5);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x6D => {
                        // BIT 5,L 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_l(), 5);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x6E => {
                        // BIT 5,(HL) 2 16 | Z 0 1 -
                        let is_bit = is_bit(self.read_hl()?, 5);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x6F => {
                        // BIT 5,A 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_a(), 5);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x70 => {
                        // BIT 6,B 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_b(), 6);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x71 => {
                        // BIT 6,C 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_c(), 6);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x72 => {
                        // BIT 6,D 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_d(), 6);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x73 => {
                        // BIT 6,E 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_e(), 6);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x74 => {
                        // BIT 6,H 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_h(), 6);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x75 => {
                        // BIT 6,L 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_l(), 6);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x76 => {
                        // BIT 6,(HL) 2 16 | Z 0 1 -
                        let is_bit = is_bit(self.read_hl()?, 6);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x77 => {
                        // BIT 6,A 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_a(), 6);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x78 => {
                        // BIT 7,B 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_b(), 7);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x79 => {
                        // BIT 7,C 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_c(), 7);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x7A => {
                        // BIT 7,D 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_d(), 7);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x7B => {
                        // BIT 7,E 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_e(), 7);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x7C => {
                        // BIT 7,H 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_h(), 7);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x7D => {
                        // BIT 7,L 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_l(), 7);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x7E => {
                        // BIT 7,(HL) 2 16 | Z 0 1 -
                        let is_bit = is_bit(self.read_hl()?, 7);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x7F => {
                        // BIT 7,A 2 8 | Z 0 1 -
                        let is_bit = is_bit(self.cpu.get_a(), 7);
                        self.cpu.set_fz(!is_bit);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(true);
                    }
                    0x80 => {
                        // RES 0,B 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_b(), 0, false);
                        self.cpu.set_b(byte);
                    }
                    0x81 => {
                        // RES 0,C 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_c(), 0, false);
                        self.cpu.set_c(byte);
                    }
                    0x82 => {
                        // RES 0,D 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_d(), 0, false);
                        self.cpu.set_d(byte);
                    }
                    0x83 => {
                        // RES 0,E 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_e(), 0, false);
                        self.cpu.set_e(byte);
                    }
                    0x84 => {
                        // RES 0,H 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_h(), 0, false);
                        self.cpu.set_h(byte);
                    }
                    0x85 => {
                        // RES 0,L 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_l(), 0, false);
                        self.cpu.set_l(byte);
                    }
                    0x86 => {
                        // RES 0,(HL) 2 16 | - - - -
                        let byte = set_bit(self.read_hl()?, 0, false);
                        self.write_hl(byte)?;
                    }
                    0x87 => {
                        // RES 0,A 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_a(), 0, false);
                        self.cpu.set_a(byte);
                    }
                    0x88 => {
                        // RES 1,B 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_b(), 1, false);
                        self.cpu.set_b(byte);
                    }
                    0x89 => {
                        // RES 1,C 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_c(), 1, false);
                        self.cpu.set_c(byte);
                    }
                    0x8A => {
                        // RES 1,D 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_d(), 1, false);
                        self.cpu.set_d(byte);
                    }
                    0x8B => {
                        // RES 1,E 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_e(), 1, false);
                        self.cpu.set_e(byte);
                    }
                    0x8C => {
                        // RES 1,H 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_h(), 1, false);
                        self.cpu.set_h(byte);
                    }
                    0x8D => {
                        // RES 1,L 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_l(), 1, false);
                        self.cpu.set_l(byte);
                    }
                    0x8E => {
                        // RES 1,(HL) 2 16 | - - - -
                        let byte = set_bit(self.read_hl()?, 1, false);
                        self.write_hl(byte)?;
                    }
                    0x8F => {
                        // RES 1,A 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_a(), 1, false);
                        self.cpu.set_a(byte);
                    }
                    0x90 => {
                        // RES 2,B 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_b(), 2, false);
                        self.cpu.set_b(byte);
                    }
                    0x91 => {
                        // RES 2,C 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_c(), 2, false);
                        self.cpu.set_c(byte);
                    }
                    0x92 => {
                        // RES 2,D 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_d(), 2, false);
                        self.cpu.set_d(byte);
                    }
                    0x93 => {
                        // RES 2,E 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_e(), 2, false);
                        self.cpu.set_e(byte);
                    }
                    0x94 => {
                        // RES 2,H 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_h(), 2, false);
                        self.cpu.set_h(byte);
                    }
                    0x95 => {
                        // RES 2,L 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_l(), 2, false);
                        self.cpu.set_l(byte);
                    }
                    0x96 => {
                        // RES 2,(HL) 2 16 | - - - -
                        let byte = set_bit(self.read_hl()?, 2, false);
                        self.write_hl(byte)?;
                    }
                    0x97 => {
                        // RES 2,A 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_a(), 2, false);
                        self.cpu.set_a(byte);
                    }
                    0x98 => {
                        // RES 3,B 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_b(), 3, false);
                        self.cpu.set_b(byte);
                    }
                    0x99 => {
                        // RES 3,C 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_c(), 3, false);
                        self.cpu.set_c(byte);
                    }
                    0x9A => {
                        // RES 3,D 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_d(), 3, false);
                        self.cpu.set_d(byte);
                    }
                    0x9B => {
                        // RES 3,E 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_e(), 3, false);
                        self.cpu.set_e(byte);
                    }
                    0x9C => {
                        // RES 3,H 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_h(), 3, false);
                        self.cpu.set_h(byte);
                    }
                    0x9D => {
                        // RES 3,L 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_l(), 3, false);
                        self.cpu.set_l(byte);
                    }
                    0x9E => {
                        // RES 3,(HL) 2 16 | - - - -
                        let byte = set_bit(self.read_hl()?, 3, false);
                        self.write_hl(byte)?;
                    }
                    0x9F => {
                        // RES 3,A 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_a(), 3, false);
                        self.cpu.set_a(byte);
                    }
                    0xA0 => {
                        // RES 4,B 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_b(), 4, false);
                        self.cpu.set_b(byte);
                    }
                    0xA1 => {
                        // RES 4,C 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_c(), 4, false);
                        self.cpu.set_c(byte);
                    }
                    0xA2 => {
                        // RES 4,D 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_d(), 4, false);
                        self.cpu.set_d(byte);
                    }
                    0xA3 => {
                        // RES 4,E 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_e(), 4, false);
                        self.cpu.set_e(byte);
                    }
                    0xA4 => {
                        // RES 4,H 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_h(), 4, false);
                        self.cpu.set_h(byte);
                    }
                    0xA5 => {
                        // RES 4,L 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_l(), 4, false);
                        self.cpu.set_l(byte);
                    }
                    0xA6 => {
                        // RES 4,(HL) 2 16 | - - - -
                        let byte = set_bit(self.read_hl()?, 4, false);
                        self.write_hl(byte)?;
                    }
                    0xA7 => {
                        // RES 4,A 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_a(), 4, false);
                        self.cpu.set_a(byte);
                    }
                    0xA8 => {
                        // RES 5,B 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_b(), 5, false);
                        self.cpu.set_b(byte);
                    }
                    0xA9 => {
                        // RES 5,C 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_c(), 5, false);
                        self.cpu.set_c(byte);
                    }
                    0xAA => {
                        // RES 5,D 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_d(), 5, false);
                        self.cpu.set_d(byte);
                    }
                    0xAB => {
                        // RES 5,E 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_e(), 5, false);
                        self.cpu.set_e(byte);
                    }
                    0xAC => {
                        // RES 5,H 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_h(), 5, false);
                        self.cpu.set_h(byte);
                    }
                    0xAD => {
                        // RES 5,L 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_l(), 5, false);
                        self.cpu.set_l(byte);
                    }
                    0xAE => {
                        // RES 5,(HL) 2 16 | - - - -
                        let byte = set_bit(self.read_hl()?, 5, false);
                        self.write_hl(byte)?;
                    }
                    0xAF => {
                        // RES 5,A 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_a(), 5, false);
                        self.cpu.set_a(byte);
                    }
                    0xB0 => {
                        // RES 6,B 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_b(), 6, false);
                        self.cpu.set_b(byte);
                    }
                    0xB1 => {
                        // RES 6,C 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_c(), 6, false);
                        self.cpu.set_c(byte);
                    }
                    0xB2 => {
                        // RES 6,D 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_d(), 6, false);
                        self.cpu.set_d(byte);
                    }
                    0xB3 => {
                        // RES 6,E 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_e(), 6, false);
                        self.cpu.set_e(byte);
                    }
                    0xB4 => {
                        // RES 6,H 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_h(), 6, false);
                        self.cpu.set_h(byte);
                    }
                    0xB5 => {
                        // RES 6,L 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_l(), 6, false);
                        self.cpu.set_l(byte);
                    }
                    0xB6 => {
                        // RES 6,(HL) 2 16 | - - - -
                        let byte = set_bit(self.read_hl()?, 6, false);
                        self.write_hl(byte)?;
                    }
                    0xB7 => {
                        // RES 6,A 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_a(), 6, false);
                        self.cpu.set_a(byte);
                    }
                    0xB8 => {
                        // RES 7,B 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_b(), 7, false);
                        self.cpu.set_b(byte);
                    }
                    0xB9 => {
                        // RES 7,C 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_c(), 7, false);
                        self.cpu.set_c(byte);
                    }
                    0xBA => {
                        // RES 7,D 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_d(), 7, false);
                        self.cpu.set_d(byte);
                    }
                    0xBB => {
                        // RES 7,E 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_e(), 7, false);
                        self.cpu.set_e(byte);
                    }
                    0xBC => {
                        // RES 7,H 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_h(), 7, false);
                        self.cpu.set_h(byte);
                    }
                    0xBD => {
                        // RES 7,L 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_l(), 7, false);
                        self.cpu.set_l(byte);
                    }
                    0xBE => {
                        // RES 7,(HL) 2 16 | - - - -
                        let byte = set_bit(self.read_hl()?, 7, false);
                        self.write_hl(byte)?;
                    }
                    0xBF => {
                        // RES 7,A 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_a(), 7, false);
                        self.cpu.set_a(byte);
                    }
                    0xC0 => {
                        // SET 0,B 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_b(), 0, true);
                        self.cpu.set_b(byte);
                    }
                    0xC1 => {
                        // SET 0,C 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_c(), 0, true);
                        self.cpu.set_c(byte);
                    }
                    0xC2 => {
                        // SET 0,D 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_d(), 0, true);
                        self.cpu.set_d(byte);
                    }
                    0xC3 => {
                        // SET 0,E 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_e(), 0, true);
                        self.cpu.set_e(byte);
                    }
                    0xC4 => {
                        // SET 0,H 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_h(), 0, true);
                        self.cpu.set_h(byte);
                    }
                    0xC5 => {
                        // SET 0,L 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_l(), 0, true);
                        self.cpu.set_l(byte);
                    }
                    0xC6 => {
                        // SET 0,(HL) 2 16 | - - - -
                        let byte = set_bit(self.read_hl()?, 0, true);
                        self.write_hl(byte)?;
                    }
                    0xC7 => {
                        // SET 0,A 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_a(), 0, true);
                        self.cpu.set_a(byte);
                    }
                    0xC8 => {
                        // SET 1,B 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_b(), 1, true);
                        self.cpu.set_b(byte);
                    }
                    0xC9 => {
                        // SET 1,C 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_c(), 1, true);
                        self.cpu.set_c(byte);
                    }
                    0xCA => {
                        // SET 1,D 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_d(), 1, true);
                        self.cpu.set_d(byte);
                    }
                    0xCB => {
                        // SET 1,E 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_e(), 1, true);
                        self.cpu.set_e(byte);
                    }
                    0xCC => {
                        // SET 1,H 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_h(), 1, true);
                        self.cpu.set_h(byte);
                    }
                    0xCD => {
                        // SET 1,L 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_l(), 1, true);
                        self.cpu.set_l(byte);
                    }
                    0xCE => {
                        // SET 1,(HL) 2 16 | - - - -
                        let byte = set_bit(self.read_hl()?, 1, true);
                        self.write_hl(byte)?;
                    }
                    0xCF => {
                        // SET 1,A 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_a(), 1, true);
                        self.cpu.set_a(byte);
                    }
                    0xD0 => {
                        // SET 2,B 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_b(), 2, true);
                        self.cpu.set_b(byte);
                    }
                    0xD1 => {
                        // SET 2,C 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_c(), 2, true);
                        self.cpu.set_c(byte);
                    }
                    0xD2 => {
                        // SET 2,D 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_d(), 2, true);
                        self.cpu.set_d(byte);
                    }
                    0xD3 => {
                        // SET 2,E 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_e(), 2, true);
                        self.cpu.set_e(byte);
                    }
                    0xD4 => {
                        // SET 2,H 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_h(), 2, true);
                        self.cpu.set_h(byte);
                    }
                    0xD5 => {
                        // SET 2,L 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_l(), 2, true);
                        self.cpu.set_l(byte);
                    }
                    0xD6 => {
                        // SET 2,(HL) 2 16 | - - - -
                        let byte = set_bit(self.read_hl()?, 2, true);
                        self.write_hl(byte)?;
                    }
                    0xD7 => {
                        // SET 2,A 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_a(), 2, true);
                        self.cpu.set_a(byte);
                    }
                    0xD8 => {
                        // SET 3,B 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_b(), 3, true);
                        self.cpu.set_b(byte);
                    }
                    0xD9 => {
                        // SET 3,C 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_c(), 3, true);
                        self.cpu.set_c(byte);
                    }
                    0xDA => {
                        // SET 3,D 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_d(), 3, true);
                        self.cpu.set_d(byte);
                    }
                    0xDB => {
                        // SET 3,E 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_e(), 3, true);
                        self.cpu.set_e(byte);
                    }
                    0xDC => {
                        // SET 3,H 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_h(), 3, true);
                        self.cpu.set_h(byte);
                    }
                    0xDD => {
                        // SET 3,L 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_l(), 3, true);
                        self.cpu.set_l(byte);
                    }
                    0xDE => {
                        // SET 3,(HL) 2 16 | - - - -
                        let byte = set_bit(self.read_hl()?, 3, true);
                        self.write_hl(byte)?;
                    }
                    0xDF => {
                        // SET 3,A 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_a(), 3, true);
                        self.cpu.set_a(byte);
                    }
                    0xE0 => {
                        // SET 4,B 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_b(), 4, true);
                        self.cpu.set_b(byte);
                    }
                    0xE1 => {
                        // SET 4,C 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_c(), 4, true);
                        self.cpu.set_c(byte);
                    }
                    0xE2 => {
                        // SET 4,D 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_d(), 4, true);
                        self.cpu.set_d(byte);
                    }
                    0xE3 => {
                        // SET 4,E 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_e(), 4, true);
                        self.cpu.set_e(byte);
                    }
                    0xE4 => {
                        // SET 4,H 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_h(), 4, true);
                        self.cpu.set_h(byte);
                    }
                    0xE5 => {
                        // SET 4,L 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_l(), 4, true);
                        self.cpu.set_l(byte);
                    }
                    0xE6 => {
                        // SET 4,(HL) 2 16 | - - - -
                        let byte = set_bit(self.read_hl()?, 4, true);
                        self.write_hl(byte)?;
                    }
                    0xE7 => {
                        // SET 4,A 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_a(), 4, true);
                        self.cpu.set_a(byte);
                    }
                    0xE8 => {
                        // SET 5,B 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_b(), 5, true);
                        self.cpu.set_b(byte);
                    }
                    0xE9 => {
                        // SET 5,C 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_c(), 5, true);
                        self.cpu.set_c(byte);
                    }
                    0xEA => {
                        // SET 5,D 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_d(), 5, true);
                        self.cpu.set_d(byte);
                    }
                    0xEB => {
                        // SET 5,E 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_e(), 5, true);
                        self.cpu.set_e(byte);
                    }
                    0xEC => {
                        // SET 5,H 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_h(), 5, true);
                        self.cpu.set_h(byte);
                    }
                    0xED => {
                        // SET 5,L 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_l(), 5, true);
                        self.cpu.set_l(byte);
                    }
                    0xEE => {
                        // SET 5,(HL) 2 16 | - - - -
                        let byte = set_bit(self.read_hl()?, 5, true);
                        self.write_hl(byte)?;
                    }
                    0xEF => {
                        // SET 5,A 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_a(), 5, true);
                        self.cpu.set_a(byte);
                    }
                    0xF0 => {
                        // SET 6,B 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_b(), 6, true);
                        self.cpu.set_b(byte);
                    }
                    0xF1 => {
                        // SET 6,C 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_c(), 6, true);
                        self.cpu.set_c(byte);
                    }
                    0xF2 => {
                        // SET 6,D 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_d(), 6, true);
                        self.cpu.set_d(byte);
                    }
                    0xF3 => {
                        // SET 6,E 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_e(), 6, true);
                        self.cpu.set_e(byte);
                    }
                    0xF4 => {
                        // SET 6,H 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_h(), 6, true);
                        self.cpu.set_h(byte);
                    }
                    0xF5 => {
                        // SET 6,L 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_l(), 6, true);
                        self.cpu.set_l(byte);
                    }
                    0xF6 => {
                        // SET 6,(HL) 2 16 | - - - -
                        let byte = set_bit(self.read_hl()?, 6, true);
                        self.write_hl(byte)?;
                    }
                    0xF7 => {
                        // SET 6,A 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_a(), 6, true);
                        self.cpu.set_a(byte);
                    }
                    0xF8 => {
                        // SET 7,B 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_b(), 7, true);
                        self.cpu.set_b(byte);
                    }
                    0xF9 => {
                        // SET 7,C 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_c(), 7, true);
                        self.cpu.set_c(byte);
                    }
                    0xFA => {
                        // SET 7,D 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_d(), 7, true);
                        self.cpu.set_d(byte);
                    }
                    0xFB => {
                        // SET 7,E 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_e(), 7, true);
                        self.cpu.set_e(byte);
                    }
                    0xFC => {
                        // SET 7,H 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_h(), 7, true);
                        self.cpu.set_h(byte);
                    }
                    0xFD => {
                        // SET 7,L 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_l(), 7, true);
                        self.cpu.set_l(byte);
                    }
                    0xFE => {
                        // SET 7,(HL) 2 16 | - - - -
                        let byte = set_bit(self.read_hl()?, 7, true);
                        self.write_hl(byte)?;
                    }
                    0xFF => {
                        // SET 7,A 2 8 | - - - -
                        let byte = set_bit(self.cpu.get_a(), 7, true);
                        self.cpu.set_a(byte);
                    }
                };

                self.tick(OPCODE_MCYCLE_PREFIX[op_cb as usize]);
            }
            0xCC => {
                // CALL Z,a16 3 24/12 | - - - -
                let addr = self.read_op_imm16()?;

                if self.cpu.is_fz() {
                    self.push_u16(self.cpu.pc)?;
                    self.cpu.pc = addr;
                } else {
                    is_alternative_mcycle = true;
                }
            }
            0xCD => {
                // CALL a16 3 24 | - - - -
                let addr = self.read_op_imm16()?;

                self.push_u16(self.cpu.pc)?;
                self.cpu.pc = addr;
            }
            0xCE => {
                // ADC A,d8 2 8 | Z 0 H C
                let byte = self.read_op()?.wrapping_add(self.cpu.get_fc());
                self.cpu.add(byte);
            }
            0xCF => {
                // RST 08H 1 16 | - - - -
                self.push_u16(self.cpu.pc)?;
                self.cpu.pc = (op - 0xC7) as u16;
            }
            0xD0 => {
                // RET NC 1 20/8 | - - - -
                if !self.cpu.is_fc() {
                    let addr = self.pop_u16()?;
                    self.cpu.pc = addr;
                } else {
                    is_alternative_mcycle = true;
                }
            }
            0xD1 => {
                // POP DE 1 12 | - - - -
                self.cpu.de = self.pop_u16()?;
            }
            0xD2 => {
                // JP NC,a16 3 16/12 | - - - -
                let addr = self.read_op_imm16()?;
                if !self.cpu.is_fc() {
                    self.cpu.pc = addr;
                } else {
                    is_alternative_mcycle = true;
                }
            }
            0xD3 => panic!("Opcode 0xD3 is invalid"),
            0xD4 => {
                // CALL NC,a16 3 24/12 | - - - -
                let addr = self.read_op_imm16()?;

                if !self.cpu.is_fc() {
                    self.push_u16(self.cpu.pc)?;
                    self.cpu.pc = addr;
                } else {
                    is_alternative_mcycle = true;
                }
            }
            0xD5 => {
                // PUSH DE 1 16 | - - - -
                self.push_u16(self.cpu.de)?;
            }
            0xD6 => {
                // SUB d8 2 8 | Z 1 H C
                let byte = self.read_op()?;
                self.cpu.sub(byte);
            }
            0xD7 => {
                // RST 10H 1 16 | - - - -
                self.push_u16(self.cpu.pc)?;
                self.cpu.pc = (op - 0xC7) as u16;
            }
            0xD8 => {
                // RET C 1 20/8 | - - - -
                if self.cpu.is_fc() {
                    let addr = self.pop_u16()?;
                    self.cpu.pc = addr;
                } else {
                    is_alternative_mcycle = true;
                }
            }
            0xD9 => {
                // RETI 1 16 | - - - -
                let addr = self.pop_u16()?;
                self.cpu.pc = addr;
                self.mem.write(MEM_LOC_IE, 1)?;
            }
            0xDA => {
                // JP C,a16 3 16/12 | - - - -
                let addr = self.read_op_imm16()?;
                if self.cpu.is_fc() {
                    self.cpu.pc = addr;
                } else {
                    is_alternative_mcycle = true;
                }
            }
            0xDB => panic!("Opcode 0xDB is invalid"),
            0xDC => {
                // CALL C,a16 3 24/12 | - - - -
                let addr = self.read_op_imm16()?;

                if self.cpu.is_fc() {
                    self.push_u16(self.cpu.pc)?;
                    self.cpu.pc = addr;
                } else {
                    is_alternative_mcycle = true;
                }
            }
            0xDD => panic!("Opcode 0xDD is invalid"),
            0xDE => {
                // SBC A,d8 2 8 | Z 1 H C
                let byte = self.read_op()?;
                self.cpu.sub(byte);
            }
            0xDF => {
                // RST 18H 1 16 | - - - -
                self.push_u16(self.cpu.pc)?;
                self.cpu.pc = (op - 0xC7) as u16;
            }
            0xE0 => {
                // LDH (a8),A 2 12 | - - - -
                let byte = self.cpu.get_a();
                let word = 0xFF00u16 | self.read_op()? as u16;
                self.mem.write(word, byte)?;
            }
            0xE1 => {
                // POP HL 1 12 | - - - -
                self.cpu.hl = self.pop_u16()?;
            }
            0xE2 => {
                // LD (C),A 2 8 | - - - -
                let byte = self.cpu.get_a();
                let word = 0xFF00u16 | self.cpu.get_c() as u16;
                self.mem.write(word, byte)?;
            }
            0xE3 => panic!("Opcode 0xE3 is invalid"),
            0xE4 => panic!("Opcode 0xE4 is invalid"),
            0xE5 => {
                // PUSH HL 1 16 | - - - -
                self.push_u16(self.cpu.hl)?;
            }
            0xE6 => {
                // AND d8 2 8 | Z 0 1 0
                let byte = self.read_op()?;
                self.cpu.and(byte);
            }
            0xE7 => {
                // RST 20H 1 16 | - - - -
                self.push_u16(self.cpu.pc)?;
                self.cpu.pc = (op - 0xC7) as u16;
            }
            0xE8 => {
                // ADD SP,r8 2 16 | 0 0 H C
                let offs = self.read_op()? as i8;
                let word = wrapping_add_u16_i8(self.cpu.sp, offs);
                let is_carry;
                let is_half_carry;

                is_carry = is_carry_add_u16(self.cpu.sp, word as u16);
                is_half_carry = is_half_carry_add_u16(self.cpu.sp, word as u16);
                self.cpu.sp = self.cpu.sp.wrapping_add(word as u16);

                self.cpu.set_fc(is_carry);
                self.cpu.set_fh(is_half_carry);
                self.cpu.set_fz(false);
                self.cpu.set_fn(false);
            }
            0xE9 => {
                // JP (HL) 1 4 | - - - -
                let addr = self.cpu.hl;
                self.cpu.pc = addr;
            }
            0xEA => {
                // LD (a16),A 3 16 | - - - -
                let word = self.read_op_imm16()?;
                let byte = self.cpu.get_a();
                self.mem.write(word, byte)?;
            }
            0xEB => panic!("Opcode 0xEB is invalid"),
            0xEC => panic!("Opcode 0xEC is invalid"),
            0xED => panic!("Opcode 0xED is invalid"),
            0xEE => {
                // XOR d8 2 8 | Z 0 0 0
                let byte = self.read_op()?;
                self.cpu.xor(byte);
            }
            0xEF => {
                // RST 28H 1 16 | - - - -
                self.push_u16(self.cpu.pc)?;
                self.cpu.pc = (op - 0xC7) as u16;
            }
            0xF0 => {
                // LDH A,(a8) 2 12 | - - - -
                let word = 0xFF00u16 | self.read_op()? as u16;
                let byte = self.mem.read(word)?;
                self.cpu.set_a(byte);
            }
            0xF1 => {
                // POP AF 1 12 | Z N H C
                self.cpu.af = self.pop_u16()?;
            }
            0xF2 => {
                // LD A,(C) 2 8 | - - - -
                let word = 0xFF00u16 | self.cpu.get_c() as u16;
                let byte = self.mem.read(word)?;
                self.cpu.set_a(byte);
            }
            0xF3 => {
                // DI 1 4 | - - - -
                self.mem.write(MEM_LOC_IE, 0)?;
            }
            0xF4 => panic!("Opcode 0xF4 is invalid"),
            0xF5 => {
                // PUSH AF 1 16 | - - - -
                self.push_u16(self.cpu.af)?;
            }
            0xF6 => {
                // OR d8 2 8 | Z 0 0 0
                let byte = self.read_op()?;
                self.cpu.or(byte);
            }
            0xF7 => {
                // RST 30H 1 16 | - - - -
                self.push_u16(self.cpu.pc)?;
                self.cpu.pc = (op - 0xC7) as u16;
            }
            0xF8 => {
                // LD HL,SP+r8 2 12 | 0 0 H C
                let offs = self.read_op()? as i8;
                let word = wrapping_add_u16_i8(self.cpu.sp, offs);
                let is_carry;
                let is_half_carry;

                is_carry = is_carry_add_u16(self.cpu.sp, word as u16);
                is_half_carry = is_half_carry_add_u16(self.cpu.sp, word as u16);
                self.cpu.hl = self.cpu.sp.wrapping_add(word as u16);

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
                self.mem.write(MEM_LOC_IE, 1)?;
            }
            0xFC => panic!("Opcode 0xFC is invalid"),
            0xFD => panic!("Opcode 0xFD is invalid"),
            0xFE => {
                // CP d8 2 8 | Z 1 H C
                let byte = self.read_op()?;
                self.cpu.cp(byte);
            }
            0xFF => {
                // RST 38H 1 16 | - - - -
                self.push_u16(self.cpu.pc)?;
                self.cpu.pc = (op - 0xC7) as u16;
            }
        };

        if is_alternative_mcycle {
            self.tick(OPCODE_MCYCLE_ALT[op as usize]);
        } else {
            self.tick(OPCODE_MCYCLE[op as usize]);
        }

        Ok(())
    }

    fn read_op(&mut self) -> Result<u8, Error> {
        let op = self.mem.read(self.cpu.pc)?;
        self.cpu.pc = self.cpu.pc.wrapping_add(1);

        Ok(op)
    }

    fn read_hl(&self) -> Result<u8, Error> {
        self.mem.read(self.cpu.hl)
    }

    fn write_hl(&mut self, byte: u8) -> Result<(), Error> {
        self.mem.write(self.cpu.hl, byte)
    }

    fn read_op_imm16(&mut self) -> Result<u16, Error> {
        let lo = self.read_op()?;
        let hi = self.read_op()?;

        Ok(((hi as u16) << 8) | lo as u16)
    }

    fn push_u16(&mut self, word: u16) -> Result<(), Error> {
        self.cpu.sp = self.cpu.sp.wrapping_sub(2);
        self.mem.write_u16(self.cpu.sp, word)
    }

    fn pop_u16(&mut self) -> Result<u16, Error> {
        let word = self.mem.read_u16(self.cpu.sp)?;
        self.cpu.sp = self.cpu.sp.wrapping_add(2);
        Ok(word)
    }

    fn read_repl(&self) -> Result<Option<DebugCmd>, Error> {
        let next_op = self.mem.read(self.cpu.pc)?;
        if next_op == 0xCB {
            let next_prefix_op = self.mem.read(self.cpu.pc + 1)?;

            print!(
                "{:>8} | NXT {:#04X} | {} > ",
                self.counter,
                self.cpu.pc + 1,
                OPCODE_CB_NAME[next_prefix_op as usize]
            );
        } else {
            print!(
                "{:>8} | NXT {:#04X} | {} > ",
                self.counter, self.cpu.pc, OPCODE_NAME[next_op as usize]
            );
        }

        let mut buf = String::new();
        stdout().flush()?;
        stdin().read_line(&mut buf)?;

        Ok(DebugCmd::parse(buf))
    }

    fn print_debug_panel(&self) {
        println!("+---");
        println!(
            "| A {:02X} {:02X} F | Z:{} N:{} H:{} C:{}",
            self.cpu.get_a(),
            self.cpu.get_f(),
            self.cpu.get_fz(),
            self.cpu.get_fn(),
            self.cpu.get_fh(),
            self.cpu.get_fc()
        );
        println!("| B {:02X} {:02X} C", self.cpu.get_b(), self.cpu.get_c());
        println!("| D {:02X} {:02X} E", self.cpu.get_d(), self.cpu.get_e());
        println!("| H {:02X} {:02X} L", self.cpu.get_h(), self.cpu.get_l());
        println!("| SP: {:#06X} PC: {:#06X}", self.cpu.sp, self.cpu.pc);
        println!("+---");
    }

    fn tick(&mut self, add: u8) {
        self.cpu.mcycle += add as u64;
        self.div_ticker += add as u16;
        self.tima_ticker += add as u32;
    }

    fn handle_ticks(&mut self, old_tma: u8) -> Result<(), Error> {
        if self.div_ticker >= DIV_REG_UPDATE_PER_MCYCLE {
            self.div_ticker -= DIV_REG_UPDATE_PER_MCYCLE;
            self.mem
                .write_unchecked(MEM_LOC_DIV, self.mem.read(MEM_LOC_DIV)?.wrapping_add(1))?;
        }

        let (timer_enable, timer_clock) = self.tac()?;
        if timer_enable {
            if self.tima_ticker >= timer_clock {
                self.tima_ticker -= timer_clock;

                let tima = self.mem.read_unchecked(MEM_LOC_TIMA)?;
                if tima == u8::MAX {
                    self.mem.write_unchecked(MEM_LOC_TIMA, old_tma)?;
                    unimplemented!("TIMA interrupt not implemented")
                } else {
                    self.mem
                        .write_unchecked(MEM_LOC_TIMA, tima.wrapping_add(1))?;
                }
            }
        }

        Ok(())
    }

    fn tac(&self) -> Result<(bool, u32), Error> {
        let tac = self.mem.read_unchecked(MEM_LOC_TAC)?;

        let timer_enable = (tac & 0b100) > 0;
        let timer_clock = TIMA_UPDATE_PER_MCYCLE[(tac & 0b11) as usize];

        Ok((timer_enable, timer_clock))
    }
}
