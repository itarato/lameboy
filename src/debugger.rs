use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use log::info;

pub enum DebugCmd {
    Quit,
    Continue,
    Step,
    PrintCpu,
    PrintMemory(u16, usize),
    PrintOpHistory,
    PrintOam,
}

pub struct Debugger {
    break_on_start: bool,
    step_by_step: bool,
    pc_breakpoints: Vec<u16>,
    auto_step_count: usize,
    one_time_break: bool,
    breakpoint_requested: Rc<RefCell<bool>>,
}

impl Debugger {
    pub fn new(breakpoint_requested: Rc<RefCell<bool>>) -> Self {
        Self {
            break_on_start: false,
            step_by_step: false,
            pc_breakpoints: vec![],
            auto_step_count: 0,
            one_time_break: false,
            breakpoint_requested,
        }
    }

    pub fn parse(&mut self, raw: String) -> Option<DebugCmd> {
        let raw = raw.trim();
        let parts = raw.split(" ").collect::<Vec<&str>>();

        if raw == "q" {
            Some(DebugCmd::Quit)
        } else if raw == "" {
            self.set_auto_step_count(0);
            Some(DebugCmd::Step)
        } else if parts[0] == "n" {
            let auto_step = if parts.len() > 1 {
                usize::from_str_radix(parts[1], 10).unwrap_or(1)
            } else {
                1
            };

            self.set_auto_step_count(auto_step - 1);
            Some(DebugCmd::Step)
        } else if raw == "p" {
            Some(DebugCmd::PrintCpu)
        } else if raw == "c" {
            Some(DebugCmd::Continue)
        } else if parts.len() == 2 && parts[0] == "b" {
            usize::from_str_radix(parts[1], 16)
                .ok()
                .map(|pc| self.add_breakpoint(pc as u16));
            self.dump_breakpoints();
            None
        } else if raw == "b?" {
            self.dump_breakpoints();
            None
        } else if parts[0] == "b-" {
            if parts.len() == 1 {
                self.pc_breakpoints.clear();
            } else {
                for i in 1..parts.len() {
                    if let Some(i) = u16::from_str_radix(parts[i], 16)
                        .ok()
                        .and_then(|v_in| self.pc_breakpoints.iter().position(|e| e == &v_in))
                    {
                        self.pc_breakpoints.remove(i);
                    }
                }
            }
            self.dump_breakpoints();
            None
        } else if raw == "s" {
            self.set_step_by_step();
            None
        } else if raw == "hist" {
            Some(DebugCmd::PrintOpHistory)
        } else if parts.len() >= 2 && parts[0] == "m" {
            u16::from_str_radix(parts[1], 16)
                .and_then(|from| {
                    if parts.len() == 2 {
                        Ok(DebugCmd::PrintMemory(from, 1))
                    } else {
                        usize::from_str_radix(parts[2], 10)
                            .map(|len| DebugCmd::PrintMemory(from, len))
                    }
                })
                .ok()
        } else if raw == "oam" {
            Some(DebugCmd::PrintOam)
        } else {
            println!("Invalid debug command: {}", raw);
            None
        }
    }

    pub fn clear_steps_and_continue(&mut self) {
        self.auto_step_count = 0;
        self.step_by_step = false;
    }

    pub fn set_auto_step_count(&mut self, n: usize) {
        self.auto_step_count = n;
    }

    pub fn set_break_on_start(&mut self) {
        self.break_on_start = true;
    }

    pub fn set_step_by_step(&mut self) {
        self.step_by_step = true;
    }

    pub fn add_breakpoint(&mut self, breakpoint: u16) {
        info!("Breakpoint has been added: {:?}", breakpoint);
        self.pc_breakpoints.push(breakpoint);
    }

    #[allow(dead_code)]
    pub fn request_one_time_break(&mut self) {
        self.one_time_break = true;
    }

    pub fn should_stop(&mut self, pc: u16) -> bool {
        if self.auto_step_count > 0 {
            self.auto_step_count -= 1;
            return false;
        }

        if self.step_by_step {
            return true;
        }

        if pc == 0 && self.break_on_start {
            return true;
        }

        if self.pc_breakpoints.contains(&pc) {
            return true;
        }

        if self.one_time_break {
            self.one_time_break = false;
            return true;
        }

        if *self.breakpoint_requested.borrow() {
            *self.breakpoint_requested.borrow_mut() = false;
            return true;
        }

        false
    }

    fn dump_breakpoints(&self) {
        let lines = self
            .pc_breakpoints
            .iter()
            .map(|v| format!("{:04X}", v))
            .collect::<Vec<_>>()
            .join(" ");
        println!("Breakpoints: {}", lines);
    }
}
