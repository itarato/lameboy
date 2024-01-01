use std::fs::File;
use std::io::stdin;
use std::io::stdout;
use std::io::Read;
use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::RwLock;

use crate::cartridge::*;
use crate::conf::*;
use crate::cpu::*;
use crate::debugger::*;
use crate::joypad;
use crate::joypad::Joypad;
use crate::mem::*;
use crate::serial::Serial;
use crate::sound::*;
use crate::timer::*;
use crate::util::*;
use crate::video::*;

enum DelayedOp {
    MasterInterruptEnable,
    MasterInterruptDisable,
}

struct DelayedCommand {
    cycle_delay: usize,
    op: DelayedOp,
}

impl DelayedCommand {
    fn new(cycle_delay: usize, op: DelayedOp) -> DelayedCommand {
        DelayedCommand { cycle_delay, op }
    }

    fn dec(&mut self) {
        self.cycle_delay -= 1;
    }

    fn is_ready(&self) -> bool {
        self.cycle_delay == 0
    }
}

#[derive(PartialEq)]
enum State {
    Running,
    // Power down CPU until an interrupt occurs. Use this when ever possible to reduce energy consumption.
    Halt,
    // Halt CPU & LCD display until button pressed.
    Stop,
}

#[derive(Debug)]
enum Interrupt {
    VBlank,
    LCD,
    Timer,
    Serial,
    Joypad,
}

impl Interrupt {
    fn addr(&self) -> u16 {
        match self {
            Interrupt::VBlank => 0x40,
            Interrupt::LCD => 0x48,
            Interrupt::Timer => 0x50,
            Interrupt::Serial => 0x58,
            Interrupt::Joypad => 0x60,
        }
    }

    fn bit(&self) -> u8 {
        match self {
            Interrupt::VBlank => 0,
            Interrupt::LCD => 1,
            Interrupt::Timer => 2,
            Interrupt::Serial => 3,
            Interrupt::Joypad => 4,
        }
    }
}

pub struct VM {
    global_exit_flag: Arc<AtomicBool>,
    mem: Mem,
    cpu: Cpu,
    serial: Serial,
    debugger: Debugger,
    counter: u64,
    state: State,
    timer: Timer,
    sound: Sound,
    joypad: Joypad,
    interrupt_master_enable_flag: bool,
    interrupt_enable: u8,
    interrupt_flag: u8,
    video: Arc<RwLock<Video>>,
    op_history: SizedQueue<(u16, u8)>,           // pc + op
    deep_op_history: SizedQueue<(u64, u16, u8)>, // counter + pc + op
    delayed_cmds: Vec<DelayedCommand>,
    opcode_dump_file: Option<File>,
}

impl VM {
    pub fn new(
        global_exit_flag: Arc<AtomicBool>,
        cartridge: Cartridge,
        debugger: Debugger,
        video: Arc<RwLock<Video>>,
        is_opcode_file_dump: bool,
        joypad: Joypad,
    ) -> Result<Self, Error> {
        let opcode_dump_file = if is_opcode_file_dump {
            Some(File::create("/tmp/lameboy_dump.txt").unwrap())
        } else {
            None
        };

        Ok(VM {
            global_exit_flag,
            mem: Mem::new(cartridge)?,
            cpu: Cpu::new(),
            serial: Serial::new(),
            debugger,
            counter: 0,
            state: State::Running,
            timer: Timer::new(),
            sound: Sound::new(),
            joypad,
            interrupt_master_enable_flag: false,
            interrupt_enable: 0,
            // Top 3 bits are unused - BGB reads them as 0b111x_xxxx.
            interrupt_flag: 0xE0,
            video,
            op_history: SizedQueue::new(128),
            deep_op_history: SizedQueue::new(128),
            delayed_cmds: vec![],
            opcode_dump_file,
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
                self.print_debug_panel();
                loop {
                    match self.read_repl()? {
                        Some(DebugCmd::Quit) => return Ok(()),
                        Some(DebugCmd::Next(auto_step)) => {
                            self.debugger.set_auto_step_count(auto_step - 1);
                            break;
                        }
                        Some(DebugCmd::PrintCpu) => self.print_debug_panel(),
                        Some(DebugCmd::PrintMemory(from, len)) => {
                            self.print_debug_memory(from, len);
                        }
                        Some(DebugCmd::Continue) => {
                            self.debugger.clear_steps_and_continue();
                            break;
                        }
                        Some(DebugCmd::PrintOpHistory) => self.dump_op_history(),
                        None => (),
                    };
                }
            }

            let old_cpu_mcycle: u64 = self.cpu.mcycle;
            let pre_exec_tma = self.mem_read(MEM_LOC_TMA)?;

            if self.state == State::Running {
                self.exec_op()?;
            } else {
                self.tick(1);
            }

            let mut delayed_cmds_to_delete = vec![];
            for (i, delayed_cmd) in self.delayed_cmds.iter_mut().enumerate() {
                delayed_cmd.dec();
                if delayed_cmd.is_ready() {
                    delayed_cmds_to_delete.push(i);

                    match delayed_cmd.op {
                        DelayedOp::MasterInterruptEnable => {
                            self.interrupt_master_enable_flag = true;
                        }
                        DelayedOp::MasterInterruptDisable => {
                            self.interrupt_master_enable_flag = false;
                        }
                    };
                }
            }
            for i in delayed_cmds_to_delete.iter().rev() {
                self.delayed_cmds.remove(*i);
            }

            let diff_mcycle: u64 = self.cpu.mcycle - old_cpu_mcycle;

            self.sound.update(diff_mcycle * CYCLE_PER_MCYCLE as u64);

            let should_call_times_interrupt = self.timer.handle_ticks(pre_exec_tma)?;
            if should_call_times_interrupt {
                self.interrupt_flag = self.interrupt_flag | 0b0100;
            }

            if self.state != State::Stop {
                let video_interrupt_mask = self
                    .video
                    .write()
                    .unwrap()
                    .update(diff_mcycle * CYCLE_PER_MCYCLE as u64);
                if video_interrupt_mask & VIDEO_RESULT_MASK_STAT_INTERRUPT > 0 {
                    self.interrupt_flag |= 0b10;
                }
                if video_interrupt_mask & VIDEO_RESULT_MASK_VBLANK_INTERRUPT > 0 {
                    self.interrupt_flag |= 0b1;
                }
            }

            if self.joypad.consume_interrupt() {
                self.interrupt_flag |= 0b1_0000;
            }

            self.check_interrupt();

            self.counter += 1;

            if self
                .global_exit_flag
                .load(std::sync::atomic::Ordering::Acquire)
            {
                break;
            }
        }

        self.dump_op_history();

        Ok(())
    }

    fn reset(&mut self) -> Result<(), Error> {
        self.mem.reset()?;
        self.video.write().unwrap().reset();

        // Byte 7/6/5: Unused.
        // Byte 0: VBlank interrupt.
        self.interrupt_flag = 0xE1;

        log::info!("VM reset");

        Ok(())
    }

    fn exec_op(&mut self) -> Result<(), Error> {
        let mut is_alternative_mcycle = false;
        let op = self.read_op()?;
        let mut iteration_mcycle = 0u8;

        self.op_history.push((self.cpu.pc - 1, op));
        if self.counter % 64 == 0 {
            self.deep_op_history
                .push((self.counter, self.cpu.pc - 1, op));
        }

        if let Some(ref mut opcode_dump_file) = self.opcode_dump_file {
            opcode_dump_file
                .write_fmt(format_args!(
                    "PC={:04X} OP={:02X} AF={:04X} BC={:04X} DE=={:04X} HL={:04X} SP={:04X}\n",
                    self.cpu.pc - 1,
                    op,
                    self.cpu.af,
                    self.cpu.bc,
                    self.cpu.de,
                    self.cpu.hl,
                    self.cpu.sp,
                ))
                .unwrap();
        }
        // if self.counter >= 2355146 {
        //     return Err("DONE".into());
        // }

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
                self.mem_write(self.cpu.bc, byte)?;
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
                let is_carry = is_carry_rot_left_u8(self.cpu.get_a());
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
                self.mem_write_u16(word, self.cpu.sp)?;
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
                let byte = self.mem_read(self.cpu.bc)?;
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
                let is_carry = is_carry_rot_right_u8(self.cpu.get_a());
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
                self.mem_write(MEM_LOC_DIV, 0)?;
            }
            0x11 => {
                // LD DE,d16 3 12 | - - - -
                let word = self.read_op_imm16()?;
                self.cpu.de = word;
            }
            0x12 => {
                // LD (DE),A 1 8 | - - - -
                let byte = self.cpu.get_a();
                self.mem_write(self.cpu.de, byte)?;
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
                let is_carry = is_carry_rot_left_u8(self.cpu.get_a());
                let new_a = (self.cpu.get_a() << 1) | self.cpu.get_fc();

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
                let byte = self.mem_read(self.cpu.de)?;
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
                let is_carry = is_carry_rot_right_u8(self.cpu.get_a());
                let new_a = (self.cpu.get_a() >> 1) | (self.cpu.get_fc() << 7);

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
                self.mem_write(word, byte)?;
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

                let new_byte = byte.wrapping_add(1);
                self.write_hl(new_byte)?;

                self.cpu.set_fz(new_byte == 0);
                self.cpu.set_fn(false);
                self.cpu.set_fh(is_half_carry);
            }
            0x35 => {
                // DEC (HL) 1 12 | Z 1 H -
                let byte = self.read_hl()?;
                let is_half_carry = is_half_carry_sub_u8(byte, 1);

                let new_byte = byte.wrapping_sub(1);
                self.write_hl(new_byte)?;

                self.cpu.set_fz(new_byte == 0);
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
                self.state = State::Halt;
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
                self.cpu.add_with_carry(self.cpu.get_b());
            }
            0x89 => {
                // ADC A,C 1 4 | Z 0 H C
                self.cpu.add_with_carry(self.cpu.get_c());
            }
            0x8A => {
                // ADC A,D 1 4 | Z 0 H C
                self.cpu.add_with_carry(self.cpu.get_d());
            }
            0x8B => {
                // ADC A,E 1 4 | Z 0 H C
                self.cpu.add_with_carry(self.cpu.get_e());
            }
            0x8C => {
                // ADC A,H 1 4 | Z 0 H C
                self.cpu.add_with_carry(self.cpu.get_h());
            }
            0x8D => {
                // ADC A,L 1 4 | Z 0 H C
                self.cpu.add_with_carry(self.cpu.get_l());
            }
            0x8E => {
                // ADC A,(HL) 1 8 | Z 0 H C
                self.cpu.add_with_carry(self.read_hl()?);
            }
            0x8F => {
                // ADC A,A 1 4 | Z 0 H C
                self.cpu.add_with_carry(self.cpu.get_a());
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
                let byte = self.cpu.get_b();
                self.cpu.sub_with_carry(byte);
            }
            0x99 => {
                // SBC A,C 1 4 | Z 1 H C
                let byte = self.cpu.get_c();
                self.cpu.sub_with_carry(byte);
            }
            0x9A => {
                // SBC A,D 1 4 | Z 1 H C
                let byte = self.cpu.get_d();
                self.cpu.sub_with_carry(byte);
            }
            0x9B => {
                // SBC A,E 1 4 | Z 1 H C
                let byte = self.cpu.get_e();
                self.cpu.sub_with_carry(byte);
            }
            0x9C => {
                // SBC A,H 1 4 | Z 1 H C
                let byte = self.cpu.get_h();
                self.cpu.sub_with_carry(byte);
            }
            0x9D => {
                // SBC A,L 1 4 | Z 1 H C
                let byte = self.cpu.get_l();
                self.cpu.sub_with_carry(byte);
            }
            0x9E => {
                // SBC A,(HL) 1 8 | Z 1 H C
                let byte = self.read_hl()?;
                self.cpu.sub_with_carry(byte);
            }
            0x9F => {
                // SBC A,A 1 4 | Z 1 H C
                let byte = self.cpu.get_a();
                self.cpu.sub_with_carry(byte);
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
                self.cpu.pc = 0x00;
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
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_b());
                        let new_b = self.cpu.get_b().rotate_left(1);

                        self.cpu.set_b(new_b);
                        self.cpu.set_fz(new_b == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x01 => {
                        // RLC C 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_c());
                        let new_c = self.cpu.get_c().rotate_left(1);

                        self.cpu.set_c(new_c);
                        self.cpu.set_fz(new_c == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x02 => {
                        // RLC D 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_d());
                        let new_d = self.cpu.get_d().rotate_left(1);

                        self.cpu.set_d(new_d);
                        self.cpu.set_fz(new_d == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x03 => {
                        // RLC E 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_e());
                        let new_e = self.cpu.get_e().rotate_left(1);

                        self.cpu.set_e(new_e);
                        self.cpu.set_fz(new_e == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x04 => {
                        // RLC H 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_h());
                        let new_h = self.cpu.get_h().rotate_left(1);

                        self.cpu.set_h(new_h);
                        self.cpu.set_fz(new_h == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x05 => {
                        // RLC L 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_l());
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

                        let is_carry = is_carry_rot_left_u8(byte);
                        let new_byte = byte.rotate_left(1);

                        self.write_hl(new_byte)?;

                        self.cpu.set_fz(new_byte == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x07 => {
                        // RLC A 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_left_u8(self.cpu.get_a());
                        let new_a = self.cpu.get_a().rotate_left(1);

                        self.cpu.set_a(new_a);
                        self.cpu.set_fz(new_a == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x08 => {
                        // RRC B 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_b());
                        let new_b = self.cpu.get_b().rotate_right(1);

                        self.cpu.set_b(new_b);
                        self.cpu.set_fz(new_b == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x09 => {
                        // RRC C 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_c());
                        let new_c = self.cpu.get_c().rotate_right(1);

                        self.cpu.set_c(new_c);
                        self.cpu.set_fz(new_c == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x0A => {
                        // RRC D 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_d());
                        let new_d = self.cpu.get_d().rotate_right(1);

                        self.cpu.set_d(new_d);
                        self.cpu.set_fz(new_d == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x0B => {
                        // RRC E 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_e());
                        let new_e = self.cpu.get_e().rotate_right(1);

                        self.cpu.set_e(new_e);
                        self.cpu.set_fz(new_e == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x0C => {
                        // RRC H 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_h());
                        let new_h = self.cpu.get_h().rotate_right(1);

                        self.cpu.set_h(new_h);
                        self.cpu.set_fz(new_h == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x0D => {
                        // RRC L 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_l());
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

                        let is_carry = is_carry_rot_right_u8(byte);
                        let new_byte = byte.rotate_right(1);

                        self.write_hl(new_byte)?;

                        self.cpu.set_fz(new_byte == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x0F => {
                        // RRC A 2 8 | Z 0 0 C
                        let is_carry = is_carry_rot_right_u8(self.cpu.get_a());
                        let new_a = self.cpu.get_a().rotate_right(1);

                        self.cpu.set_a(new_a);
                        self.cpu.set_fz(new_a == 0);
                        self.cpu.set_fn(false);
                        self.cpu.set_fh(false);
                        self.cpu.set_fc(is_carry);
                    }
                    0x10 => {
                        // RL B 2 8 | Z 0 0 C
                        self.cpu.shift_left_instrucrtion(Reg::B);
                    }
                    0x11 => {
                        // RL C 2 8 | Z 0 0 C
                        self.cpu.shift_left_instrucrtion(Reg::C);
                    }
                    0x12 => {
                        // RL D 2 8 | Z 0 0 C
                        self.cpu.shift_left_instrucrtion(Reg::D);
                    }
                    0x13 => {
                        // RL E 2 8 | Z 0 0 C
                        self.cpu.shift_left_instrucrtion(Reg::E);
                    }
                    0x14 => {
                        // RL H 2 8 | Z 0 0 C
                        self.cpu.shift_left_instrucrtion(Reg::H);
                    }
                    0x15 => {
                        // RL L 2 8 | Z 0 0 C
                        self.cpu.shift_left_instrucrtion(Reg::L);
                    }
                    0x16 => {
                        // RL (HL) 2 16 | Z 0 0 C
                        let byte = self.read_hl()?;
                        let is_carry = is_carry_rot_left_u8(byte);
                        let new_byte = (byte << 1) | self.cpu.get_fc();

                        self.write_hl(new_byte)?;
                        self.cpu.set_flags(new_byte == 0, false, false, is_carry);
                    }
                    0x17 => {
                        // RL A 2 8 | Z 0 0 C
                        self.cpu.shift_left_instrucrtion(Reg::A);
                    }
                    0x18 => {
                        // RR B 2 8 | Z 0 0 C
                        self.cpu.shift_right_instruction(Reg::B);
                    }
                    0x19 => {
                        // RR C 2 8 | Z 0 0 C
                        self.cpu.shift_right_instruction(Reg::C);
                    }
                    0x1A => {
                        // RR D 2 8 | Z 0 0 C
                        self.cpu.shift_right_instruction(Reg::D);
                    }
                    0x1B => {
                        // RR E 2 8 | Z 0 0 C
                        self.cpu.shift_right_instruction(Reg::E);
                    }
                    0x1C => {
                        // RR H 2 8 | Z 0 0 C
                        self.cpu.shift_right_instruction(Reg::H);
                    }
                    0x1D => {
                        // RR L 2 8 | Z 0 0 C
                        self.cpu.shift_right_instruction(Reg::L);
                    }
                    0x1E => {
                        // RR (HL) 2 16 | Z 0 0 C
                        let byte = self.read_hl()?;
                        let is_carry = is_carry_rot_right_u8(byte);
                        let new_byte = (byte >> 1) | (self.cpu.get_fc() << 7);

                        self.write_hl(new_byte)?;
                        self.cpu.set_flags(new_byte == 0, false, false, is_carry);
                    }
                    0x1F => {
                        // RR A 2 8 | Z 0 0 C
                        self.cpu.shift_right_instruction(Reg::A);
                    }
                    0x20 => {
                        // SLA B 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_left_u8(self.cpu.get_b());
                        let byte = shift_left_a(self.cpu.get_b());

                        self.cpu.set_b(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x21 => {
                        // SLA C 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_left_u8(self.cpu.get_c());
                        let byte = shift_left_a(self.cpu.get_c());

                        self.cpu.set_c(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x22 => {
                        // SLA D 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_left_u8(self.cpu.get_d());
                        let byte = shift_left_a(self.cpu.get_d());

                        self.cpu.set_d(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x23 => {
                        // SLA E 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_left_u8(self.cpu.get_e());
                        let byte = shift_left_a(self.cpu.get_e());

                        self.cpu.set_e(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x24 => {
                        // SLA H 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_left_u8(self.cpu.get_h());
                        let byte = shift_left_a(self.cpu.get_h());

                        self.cpu.set_h(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x25 => {
                        // SLA L 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_left_u8(self.cpu.get_l());
                        let byte = shift_left_a(self.cpu.get_l());

                        self.cpu.set_l(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x26 => {
                        // SLA (HL) 2 16 | Z 0 0 C
                        let byte = self.read_hl()?;
                        let is_carry = is_carry_shift_left_u8(byte);
                        let new_byte = shift_left_a(byte);

                        self.write_hl(new_byte)?;
                        self.cpu.set_flags(new_byte == 0, false, false, is_carry);
                    }
                    0x27 => {
                        // SLA A 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_left_u8(self.cpu.get_a());
                        let byte = shift_left_a(self.cpu.get_a());

                        self.cpu.set_a(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x28 => {
                        // SRA B 2 8 | Z 0 0 0
                        let old_byte = self.cpu.get_b();
                        let is_carry = is_bit(old_byte, 0);
                        let byte = shift_right_arithmetic_u8(old_byte);

                        self.cpu.set_b(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x29 => {
                        // SRA C 2 8 | Z 0 0 0
                        let old_byte = self.cpu.get_c();
                        let is_carry = is_bit(old_byte, 0);
                        let byte = shift_right_arithmetic_u8(old_byte);

                        self.cpu.set_c(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x2A => {
                        // SRA D 2 8 | Z 0 0 0
                        let old_byte = self.cpu.get_d();
                        let is_carry = is_bit(old_byte, 0);
                        let byte = shift_right_arithmetic_u8(old_byte);

                        self.cpu.set_d(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x2B => {
                        // SRA E 2 8 | Z 0 0 0
                        let old_byte = self.cpu.get_e();
                        let is_carry = is_bit(old_byte, 0);
                        let byte = shift_right_arithmetic_u8(old_byte);

                        self.cpu.set_e(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x2C => {
                        // SRA H 2 8 | Z 0 0 0
                        let old_byte = self.cpu.get_h();
                        let is_carry = is_bit(old_byte, 0);
                        let byte = shift_right_arithmetic_u8(old_byte);

                        self.cpu.set_h(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x2D => {
                        // SRA L 2 8 | Z 0 0 0
                        let old_byte = self.cpu.get_l();
                        let is_carry = is_bit(old_byte, 0);
                        let byte = shift_right_arithmetic_u8(old_byte);

                        self.cpu.set_l(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x2E => {
                        // SRA (HL) 2 16 | Z 0 0 0
                        let old_byte = self.read_hl()?;
                        let is_carry = is_bit(old_byte, 0);
                        let byte = shift_right_arithmetic_u8(old_byte);

                        self.write_hl(byte)?;
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x2F => {
                        // SRA A 2 8 | Z 0 0 0
                        let old_byte = self.cpu.get_a();
                        let is_carry = is_bit(old_byte, 0);
                        let byte = shift_right_arithmetic_u8(old_byte);

                        self.cpu.set_a(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
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
                        let is_carry = is_carry_shift_right_u8(self.cpu.get_b());
                        let byte = shift_right_logical(self.cpu.get_b());

                        self.cpu.set_b(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x39 => {
                        // SRL C 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_right_u8(self.cpu.get_c());
                        let byte = shift_right_logical(self.cpu.get_c());

                        self.cpu.set_c(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x3A => {
                        // SRL D 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_right_u8(self.cpu.get_d());
                        let byte = shift_right_logical(self.cpu.get_d());

                        self.cpu.set_d(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x3B => {
                        // SRL E 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_right_u8(self.cpu.get_e());
                        let byte = shift_right_logical(self.cpu.get_e());

                        self.cpu.set_e(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x3C => {
                        // SRL H 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_right_u8(self.cpu.get_h());
                        let byte = shift_right_logical(self.cpu.get_h());

                        self.cpu.set_h(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x3D => {
                        // SRL L 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_right_u8(self.cpu.get_l());
                        let byte = shift_right_logical(self.cpu.get_l());

                        self.cpu.set_l(byte);
                        self.cpu.set_flags(byte == 0, false, false, is_carry);
                    }
                    0x3E => {
                        // SRL (HL) 2 16 | Z 0 0 C
                        let byte = self.read_hl()?;
                        let is_carry = is_carry_shift_right_u8(byte);
                        let new_byte = shift_right_logical(byte);

                        self.write_hl(new_byte)?;
                        self.cpu.set_flags(new_byte == 0, false, false, is_carry);
                    }
                    0x3F => {
                        // SRL A 2 8 | Z 0 0 C
                        let is_carry = is_carry_shift_right_u8(self.cpu.get_a());
                        let byte = shift_right_logical(self.cpu.get_a());

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

                iteration_mcycle += OPCODE_MCYCLE_PREFIX[op_cb as usize];
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
                let byte = self.read_op()?;
                self.cpu.add_with_carry(byte);
            }
            0xCF => {
                // RST 08H 1 16 | - - - -
                self.push_u16(self.cpu.pc)?;
                self.cpu.pc = 0x08;
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
                self.cpu.pc = 0x10;
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
                self.interrupt_master_enable_flag = true;
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
                self.cpu.sub_with_carry(byte);
            }
            0xDF => {
                // RST 18H 1 16 | - - - -
                self.push_u16(self.cpu.pc)?;
                self.cpu.pc = 0x18;
            }
            0xE0 => {
                // LDH (a8),A 2 12 | - - - -
                let byte = self.cpu.get_a();
                let word = 0xFF00u16 | self.read_op()? as u16;
                self.mem_write(word, byte)?;
            }
            0xE1 => {
                // POP HL 1 12 | - - - -
                self.cpu.hl = self.pop_u16()?;
            }
            0xE2 => {
                // LD (C),A 2 8 | - - - -
                let byte = self.cpu.get_a();
                let word = 0xFF00u16 | self.cpu.get_c() as u16;
                self.mem_write(word, byte)?;
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
                self.cpu.pc = 0x20;
            }
            0xE8 => {
                // ADD SP,r8 2 16 | 0 0 H C
                let offs = self.read_op()? as i8;
                let word = (self.cpu.sp as i32 + offs as i32) as u16;

                let is_carry = is_carry_add_u8((self.cpu.sp & 0xFF) as u8, offs as u8);
                let is_half_carry = is_half_carry_add_u8((self.cpu.sp & 0xFF) as u8, offs as u8);
                self.cpu.sp = word;

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
                self.mem_write(word, byte)?;
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
                self.cpu.pc = 0x28;
            }
            0xF0 => {
                // LDH A,(a8) 2 12 | - - - -
                let word = 0xFF00u16 | self.read_op()? as u16;
                let byte = self.mem_read(word)?;
                self.cpu.set_a(byte);
            }
            0xF1 => {
                // POP AF 1 12 | Z N H C
                // The rightmost 4 bits of F in AF is unused and must remain 0 at all times.
                self.cpu.af = self.pop_u16()? & !0xFu16;
            }
            0xF2 => {
                // LD A,(C) 2 8 | - - - -
                let word = 0xFF00u16 | self.cpu.get_c() as u16;
                let byte = self.mem_read(word)?;
                self.cpu.set_a(byte);
            }
            0xF3 => {
                // DI 1 4 | - - - -
                self.delayed_cmds
                    .push(DelayedCommand::new(2, DelayedOp::MasterInterruptDisable));
            }
            0xF4 => panic!("Opcode 0xF4 is invalid"),
            0xF5 => {
                // PUSH AF 1 16 | - - - -
                self.push_u16(self.cpu.af & 0xFFF0)?;
            }
            0xF6 => {
                // OR d8 2 8 | Z 0 0 0
                let byte = self.read_op()?;
                self.cpu.or(byte);
            }
            0xF7 => {
                // RST 30H 1 16 | - - - -
                self.push_u16(self.cpu.pc)?;
                self.cpu.pc = 0x30;
            }
            0xF8 => {
                // LD HL,SP+r8 2 12 | 0 0 H C
                let offs = self.read_op()? as i8;
                let word = (self.cpu.sp as i32 + offs as i32) as u16;

                let is_carry = is_carry_add_u8((self.cpu.sp & 0xFF) as u8, offs as u8);
                let is_half_carry = is_half_carry_add_u8((self.cpu.sp & 0xFF) as u8, offs as u8);
                self.cpu.hl = word;

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
                let byte = self.mem_read(word)?;
                self.cpu.set_a(byte);
            }
            0xFB => {
                // EI 1 4 | - - - -
                self.delayed_cmds
                    .push(DelayedCommand::new(2, DelayedOp::MasterInterruptEnable));
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
                self.cpu.pc = 0x38;
            }
        };

        if is_alternative_mcycle {
            iteration_mcycle += OPCODE_MCYCLE_ALT[op as usize];
        } else {
            iteration_mcycle += OPCODE_MCYCLE[op as usize];
        }
        self.tick(iteration_mcycle);

        Ok(())
    }

    fn read_op(&mut self) -> Result<u8, Error> {
        let op = self.mem_read(self.cpu.pc)?;
        self.cpu.pc = self.cpu.pc.wrapping_add(1);

        Ok(op)
    }

    fn read_hl(&self) -> Result<u8, Error> {
        self.mem_read(self.cpu.hl)
    }

    fn write_hl(&mut self, byte: u8) -> Result<(), Error> {
        self.mem_write(self.cpu.hl, byte)
    }

    fn read_op_imm16(&mut self) -> Result<u16, Error> {
        let lo = self.read_op()?;
        let hi = self.read_op()?;

        Ok(((hi as u16) << 8) | lo as u16)
    }

    fn push_u16(&mut self, word: u16) -> Result<(), Error> {
        self.cpu.sp = self.cpu.sp.wrapping_sub(2);
        self.mem_write_u16(self.cpu.sp, word)
    }

    fn pop_u16(&mut self) -> Result<u16, Error> {
        let word = self.mem_read_u16(self.cpu.sp)?;
        self.cpu.sp = self.cpu.sp.wrapping_add(2);
        Ok(word)
    }

    fn read_repl(&mut self) -> Result<Option<DebugCmd>, Error> {
        let next_op = self.mem_read(self.cpu.pc)?;
        if next_op == 0xCB {
            let next_prefix_op = self.mem_read(self.cpu.pc + 1)?;

            print!(
                "{:>8} | NXT {:#04X} | {} > ",
                self.counter, self.cpu.pc, OPCODE_CB_NAME[next_prefix_op as usize]
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

        Ok(self.debugger.parse(buf))
    }

    fn print_debug_panel(&self) {
        println!();
        println!(
            "\x1B[93mA\x1B[0m {:02X} {:02X} \x1B[93mF\x1B[0m | \x1B[93mZ\x1B[0m{} \x1B[93mN\x1B[0m{} \x1B[93mH\x1B[0m{} \x1B[93mC\x1B[0m{} | \x1B[93mLCDC\x1B[0m {:02X}",
            self.cpu.get_a(),
            self.cpu.get_f(),
            self.cpu.get_fz(),
            self.cpu.get_fn(),
            self.cpu.get_fh(),
            self.cpu.get_fc(),
            self.mem_read(MEM_LOC_LCDC).unwrap(),
        );
        println!(
            "\x1B[93mB\x1B[0m {:02X} {:02X} \x1B[93mC\x1B[0m |             | \x1B[93mSTAT\x1B[0m {:02X}",
            self.cpu.get_b(),
            self.cpu.get_c(),
            self.mem_read(MEM_LOC_STAT).unwrap()
        );
        println!(
            "\x1B[93mD\x1B[0m {:02X} {:02X} \x1B[93mE\x1B[0m |             | \x1B[93mLY\x1B[0m {:02X}",
            self.cpu.get_d(),
            self.cpu.get_e(),
            self.video.read().unwrap().ly
        );
        println!(
            "\x1B[93mH\x1B[0m {:02X} {:02X} \x1B[93mL\x1B[0m |             | \x1B[93mSTAT CTR\x1B[0m {:04X}",
            self.cpu.get_h(),
            self.cpu.get_l(),
            self.video.read().unwrap().stat_counter
        );
        println!("\x1B[93mSP\x1B[0m {:04X}", self.cpu.sp);
        println!("\x1B[93mPC\x1B[0m {:04X}", self.cpu.pc);
        println!(
            "\x1B[93mIME\x1B[0m {} | \x1B[93mIE\x1B[0m {:02X} | \x1B[93mIF\x1B[0m {:02X}",
            self.interrupt_master_enable_flag, self.interrupt_enable, self.interrupt_flag
        );
        println!("\x1B[93mBIOS\x1B[0m {}", self.mem.boot_lock_reg == 0);
        println!();
    }

    fn print_debug_memory(&self, from: u16, len: usize) {
        for i in 0..len {
            if i % 8 == 0 {
                print!("\n\x1B[93m{:#06X}\x1B[0m", from + i as u16)
            }

            print!(
                " {:02X}",
                self.mem_read(from + i as u16)
                    .expect("Failed reading memory")
            );

            if i % 8 == 3 {
                print!(" ");
            }
        }

        println!("");
    }

    fn tick(&mut self, mcycles: u8) {
        self.cpu.mcycle += mcycles as u64;
        self.timer.tick(mcycles * CYCLE_PER_MCYCLE);
    }

    fn mem_write(&mut self, loc: u16, byte: u8) -> Result<(), Error> {
        log::debug!("Write: {:#06X} = #{:#04X}", loc, byte);

        if loc <= MEM_AREA_ROM_BANK_0_END {
            // Ignore for now. BGB seems to do nothing with these (eg LD (0x2000) a).
            // return Err("Cannot write to ROM (0)".into());
            self.mem.write(loc, byte)?;
        } else if loc <= MEM_AREA_ROM_BANK_N_END {
            return Err("Cannot write to ROM (N)".into());
        } else if loc <= MEM_AREA_VRAM_END {
            self.video.write().unwrap().write(loc, byte);
        } else if loc <= MEM_AREA_EXTERNAL_END {
            self.mem.write(loc, byte)?;
        } else if loc <= MEM_AREA_WRAM_END {
            self.mem.write(loc, byte)?;
        } else if loc <= MEM_AREA_ECHO_END {
            return Err("Write to MEM_AREA_ECHO is not implemented".into());
        } else if loc <= MEM_AREA_OAM_END {
            self.video.write().unwrap().write(loc, byte);
        } else if loc <= MEM_AREA_PROHIBITED_END {
            // Ignore for now. BGB seems to do nothing with these (eg LD (0xFEFF) a).
            // return Err("Write to MEM_AREA_PROHIBITED is not implemented".into());
        } else if loc <= MEM_AREA_IO_END {
            match loc {
                MEM_LOC_P1 => self.joypad.set_p1_button_selector(byte),
                MEM_LOC_SB => self.serial.set_sb(byte),
                MEM_LOC_SC => self.serial.set_sc(byte),
                // TODO: Additionally, this register is reset when executing the stop instruction,
                //       and only begins ticking again once stop mode ends.
                MEM_LOC_DIV => self.timer.set_div(),
                MEM_LOC_TIMA => self.timer.set_tima(byte),
                MEM_LOC_TMA => self.timer.set_tma(byte),
                MEM_LOC_TAC => self.timer.set_tac(byte),
                MEM_LOC_IF => self.interrupt_flag = byte | 0xE0,
                MEM_LOC_NR10..=MEM_LOC_NR52 => self.sound.write(loc, byte),
                MEM_LOC_LCDC..=MEM_LOC_WX => {
                    if loc == MEM_LOC_DMA {
                        assert!(byte <= 0xDF);
                        let addr = (byte as u16) << 8;
                        let block = (0..0xA0)
                            .map(|offs| self.mem_read(addr + offs).expect("Cannot read for DMA"))
                            .collect::<Vec<_>>();
                        self.video
                            .write()
                            .expect("Failed locking for DMA write")
                            .dma_oam_transfer(block);
                        // Not sure if we should spend 160 mcycle here.
                    } else {
                        self.video.write().unwrap().write(loc, byte);
                    }
                }
                MEM_LOC_KEY1 => unimplemented!("Write to register KEY1 is not implemented"),
                MEM_LOC_VBK => unimplemented!("Write to register VBK is not implemented"),
                MEM_LOC_BOOT_LOCK_REG => {
                    // BOOT_OFF can only transition from 0b0 to 0b1, so once 0b1 has been written, the boot ROM is
                    // permanently disabled until the next system reset. Writing 0b0 when BOOT_OFF is 0b0 has no
                    // effect and doesn’t lock the boot ROM.
                    if byte == 0b1 {
                        self.mem.boot_lock_reg = byte;
                    } else {
                        return Err("Boot lock register must only be set to 1".into());
                    }
                }
                MEM_LOC_HDMA1 => unimplemented!("Write to register HDMA1 is not implemented"),
                MEM_LOC_HDMA2 => unimplemented!("Write to register HDMA2 is not implemented"),
                MEM_LOC_HDMA3 => unimplemented!("Write to register HDMA3 is not implemented"),
                MEM_LOC_HDMA4 => unimplemented!("Write to register HDMA4 is not implemented"),
                MEM_LOC_HDMA5 => unimplemented!("Write to register HDMA5 is not implemented"),
                MEM_LOC_RP => unimplemented!("Write to register RP is not implemented"),
                MEM_LOC_BCPS => unimplemented!("Write to register BCPS is not implemented"),
                MEM_LOC_BCPD => unimplemented!("Write to register BCPD is not implemented"),
                MEM_LOC_OCPS => unimplemented!("Write to register OCPS is not implemented"),
                MEM_LOC_OCPD => unimplemented!("Write to register OCPD is not implemented"),
                MEM_LOC_SVBK => unimplemented!("Write to register SVBK is not implemented"),
                _ => {
                    // Ignore for now - BGB seems to ignore this.
                    // return Err(
                    //     format!("Write to MEM_AREA_IO is not implemented: {:#06X}", loc).into(),
                    // );
                }
            };
        } else if loc <= MEM_AREA_HRAM_END {
            self.mem.write(loc, byte)?;
        } else if loc == MEM_LOC_IE {
            self.set_interrupt_enable(byte);
        } else {
            return Err("Write outside of memory".into());
        }

        Ok(())
    }

    fn mem_write_u16(&mut self, loc: u16, word: u16) -> Result<(), Error> {
        log::debug!("Write: {:#06X} = #{:#06X}", loc, word);

        let hi = (word >> 8) as u8;
        let lo = (word & 0xFF) as u8;

        self.mem_write(loc, lo)?;
        self.mem_write(loc + 1, hi)?;

        Ok(())
    }

    fn mem_read(&self, loc: u16) -> Result<u8, Error> {
        match loc {
            // TODO: Add oam/vram read here proxy to video
            MEM_AREA_ROM_BANK_0_START..=MEM_AREA_ROM_BANK_N_END => self.mem.read(loc),
            MEM_AREA_VRAM_START..=MEM_AREA_VRAM_END => self.video.read().unwrap().read(loc),
            MEM_AREA_EXTERNAL_START..=MEM_AREA_ECHO_END => self.mem.read(loc),
            MEM_AREA_OAM_START..=MEM_AREA_OAM_END => self.mem.read(loc),
            MEM_AREA_PROHIBITED_START..=MEM_AREA_PROHIBITED_END => {
                Err(format!("Read from prohibited mem area: {:#06X}", loc).into())
            }
            MEM_AREA_IO_START..=MEM_AREA_IO_END => match loc {
                MEM_LOC_P1 => Ok(self.joypad.get_p1()),
                MEM_LOC_SB => unimplemented!("Read from register SB is not implemented"),
                MEM_LOC_SC => unimplemented!("Read from register SC is not implemented"),
                MEM_LOC_DIV => Ok(self.timer.div()),
                MEM_LOC_TIMA => Ok(self.timer.tima()),
                MEM_LOC_TMA => Ok(self.timer.tma()),
                MEM_LOC_TAC => Ok(self.timer.tac()),
                MEM_LOC_IF => Ok(self.interrupt_flag),
                MEM_LOC_NR10..=MEM_LOC_NR52 => self.sound.read(loc),
                MEM_LOC_LCDC..=MEM_LOC_WX => self.video.read().unwrap().read(loc),
                MEM_LOC_KEY1 => {
                    // FF4D — KEY1 (CGB Mode only): Prepare speed switch --> ignore.
                    Ok(0xFF)
                }
                MEM_LOC_VBK => unimplemented!("Read from register VBK is not implemented"),
                MEM_LOC_BOOT_LOCK_REG => Ok(self.mem.boot_lock_reg),
                MEM_LOC_HDMA1 => unimplemented!("Read from register HDMA1 is not implemented"),
                MEM_LOC_HDMA2 => unimplemented!("Read from register HDMA2 is not implemented"),
                MEM_LOC_HDMA3 => unimplemented!("Read from register HDMA3 is not implemented"),
                MEM_LOC_HDMA4 => unimplemented!("Read from register HDMA4 is not implemented"),
                MEM_LOC_HDMA5 => unimplemented!("Read from register HDMA5 is not implemented"),
                MEM_LOC_RP => unimplemented!("Read from register RP is not implemented"),
                MEM_LOC_BCPS => unimplemented!("Read from register BCPS is not implemented"),
                MEM_LOC_BCPD => unimplemented!("Read from register BCPD is not implemented"),
                MEM_LOC_OCPS => unimplemented!("Read from register OCPS is not implemented"),
                MEM_LOC_OCPD => unimplemented!("Read from register OCPD is not implemented"),
                MEM_LOC_SVBK => unimplemented!("Read from register SVBK is not implemented"),
                _ => unimplemented!("Read from MEM_AREA_IO is not implemented"),
            },
            MEM_AREA_HRAM_START..=MEM_AREA_HRAM_END => self.mem.read(loc),
            MEM_LOC_IE => Ok(self.interrupt_enable),
        }
    }

    fn mem_read_u16(&mut self, loc: u16) -> Result<u16, Error> {
        let lo = self.mem_read(loc)?;
        let hi = self.mem_read(loc + 1)?;
        Ok(((hi as u16) << 8) | lo as u16)
    }

    fn set_interrupt_enable(&mut self, value: u8) {
        assert!((0b1110_0000 & value) == 0);
        self.interrupt_enable = value;
    }

    fn is_vblank_interrupt_enabled(&self) -> bool {
        (self.interrupt_enable & 0b1) > 0
    }

    fn is_lcd_interrupt_enabled(&self) -> bool {
        (self.interrupt_enable & 0b10) > 0
    }

    fn is_timer_interrupt_enabled(&self) -> bool {
        (self.interrupt_enable & 0b100) > 0
    }

    fn is_serial_interrupt_enabled(&self) -> bool {
        (self.interrupt_enable & 0b1000) > 0
    }

    fn is_joypad_interrupt_enabled(&self) -> bool {
        (self.interrupt_enable & 0b1_0000) > 0
    }

    fn check_interrupt(&mut self) {
        if !self.interrupt_master_enable_flag && self.state != State::Halt {
            return;
        }

        // If an interrupt is pending, halt immediately exits, as expected, however the “halt bug”, explained below,
        // is triggered.
        if self.interrupt_flag & self.interrupt_enable == 0 {
            return;
        }

        self.state = State::Running;

        if !self.interrupt_master_enable_flag {
            return;
        }

        // If IME and IE allow the servicing of more than one of the requested interrupts,
        // the interrupt with the highest priority is serviced first. The priorities follow
        // the order of the bits in the IE and IF registers: Bit 0 (VBlank) has the highest
        // priority, and Bit 4 (Joypad) has the lowest priority.
        if is_bit(self.interrupt_flag, Interrupt::VBlank.bit())
            && self.is_vblank_interrupt_enabled()
        {
            self.interrupt(Interrupt::VBlank);
        } else if is_bit(self.interrupt_flag, Interrupt::LCD.bit())
            && self.is_lcd_interrupt_enabled()
        {
            self.interrupt(Interrupt::LCD);
        } else if is_bit(self.interrupt_flag, Interrupt::Timer.bit())
            && self.is_timer_interrupt_enabled()
        {
            self.interrupt(Interrupt::Timer);
        } else if is_bit(self.interrupt_flag, Interrupt::Serial.bit())
            && self.is_serial_interrupt_enabled()
        {
            self.interrupt(Interrupt::Serial);
        } else if is_bit(self.interrupt_flag, Interrupt::Joypad.bit())
            && self.is_joypad_interrupt_enabled()
        {
            self.interrupt(Interrupt::Joypad);
        }
    }

    pub fn dump_op_history(&self) {
        println!("Last {} ops (MOD-64):", self.deep_op_history.inner().len());
        for (counter, pc, op) in self.deep_op_history.inner() {
            println!("\t\x1B[37m#{}\x1B[0m: PC=\x1B[93m{:#06X}\x1B[0m OP=\x1B[95m{:#04X}\x1B[0m -> \x1B[96m{}\x1B[0m", counter, *pc, *op, OPCODE_NAME[*op as usize]);
        }

        println!("\n---\n");

        let op_count = self.op_history.inner().len();
        println!("Last {} op:", op_count);
        for (i, (pc, op)) in self.op_history.inner().iter().enumerate() {
            println!(
                "\t\x1B[37m#{}\x1B[0m: PC=\x1B[93m{:#06X}\x1B[0m OP=\x1B[95m{:#04X}\x1B[0m -> \x1B[96m{}\x1B[0m",
                self.counter as usize - (op_count - i + 1),
                *pc,
                *op,
                OPCODE_NAME[*op as usize]
            );
        }
    }

    fn interrupt(&mut self, interrupt: Interrupt) {
        self.interrupt_master_enable_flag = false;

        self.interrupt_flag &= !(1u8 << interrupt.bit());
        self.push_u16(self.cpu.pc).expect("Failed stacking PC");
        self.cpu.pc = interrupt.addr();
        self.tick(4);
    }
}
