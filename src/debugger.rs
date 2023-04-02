pub enum DebugCmd {
    Quit,
    Next(usize),
    Continue,
    Print,
}

impl DebugCmd {
    pub fn parse(raw: String) -> Option<DebugCmd> {
        let raw = raw.trim();
        let parts = raw.split(" ").collect::<Vec<&str>>();

        if raw == "q" {
            Some(DebugCmd::Quit)
        } else if raw == "" {
            Some(DebugCmd::Next(1))
        } else if parts[0] == "n" {
            let auto_step = if parts.len() > 1 {
                usize::from_str_radix(parts[1], 10).unwrap_or(1)
            } else {
                1
            };

            Some(DebugCmd::Next(auto_step))
        } else if raw == "p" {
            Some(DebugCmd::Print)
        } else if raw == "c" {
            Some(DebugCmd::Continue)
        } else {
            None
        }
    }
}

pub struct Debugger {
    break_on_start: bool,
    step_by_step: bool,
    pc_breakpoints: Vec<u16>,
    auto_step_count: usize,
}

impl Debugger {
    pub fn new() -> Self {
        Self {
            break_on_start: false,
            step_by_step: false,
            pc_breakpoints: vec![],
            auto_step_count: 0,
        }
    }

    pub fn clear_steps(&mut self) {
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

    pub fn add_breakpoints(&mut self, mut breakpoints: Vec<u16>) {
        self.pc_breakpoints.append(&mut breakpoints);
    }

    pub fn should_stop(&mut self, pc: u16) -> bool {
        if self.auto_step_count > 0 {
            self.auto_step_count -= 1;
            return false;
        }

        if self.step_by_step {
            return true;
        }

        if (pc == 0 || pc == 0x100) && self.break_on_start {
            return true;
        }

        if self.pc_breakpoints.contains(&pc) {
            return true;
        }

        false
    }
}