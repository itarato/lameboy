pub struct Joypad {
    p1: u8,
}

impl Joypad {
    pub fn new() -> Joypad {
        Joypad { p1: 0x3F }
    }

    pub fn set_p1(&mut self, value: u8) {
        self.p1 = value;
    }

    pub fn get_p1(&self) -> u8 {
        self.p1
    }
}
