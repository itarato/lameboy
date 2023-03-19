pub enum DebugCmd {
    Quit,
    Next,
    Print,
}

impl DebugCmd {
    pub fn parse(raw: String) -> Option<DebugCmd> {
        let raw = raw.trim();

        if raw == "q" {
            Some(DebugCmd::Quit)
        } else if raw == "n" || raw == "" {
            Some(DebugCmd::Next)
        } else if raw == "p" {
            Some(DebugCmd::Print)
        } else {
            None
        }
    }
}

pub struct Debugger {
    break_on_start: bool,
    step_by_step: bool,
    pc_breakpoints: Vec<u16>,
}

impl Debugger {
    pub fn new() -> Self {
        Self {
            break_on_start: false,
            step_by_step: false,
            pc_breakpoints: vec![],
        }
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

    pub fn should_stop(&self, pc: u16) -> bool {
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
