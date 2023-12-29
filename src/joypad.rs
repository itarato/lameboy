pub struct Joypad {
    p1: u8,
    need_interrupt: bool,
}

impl Joypad {
    pub fn new() -> Joypad {
        Joypad {
            p1: 0x3F,
            need_interrupt: false,
        }
    }

    pub fn set_p1(&mut self, value: u8) {
        // TODO: NOT SURE ABOUT ANY OF THIS
        let button_selector = value & 0b0011_0000;
        let button_request_inverted = value & 0xF;

        let old_buttons = self.p1 & 0xF;
        let new_buttons = !button_request_inverted;
        if old_buttons != new_buttons {
            self.need_interrupt = true;
        }

        let mut new_value = button_selector;
        new_value |= !button_request_inverted;

        self.p1 = new_value;
    }

    pub fn get_p1(&self) -> u8 {
        self.p1
    }

    pub fn consume_interrupt(&mut self) -> bool {
        let need_interrupt = self.need_interrupt;
        self.need_interrupt = false;
        need_interrupt
    }
}
