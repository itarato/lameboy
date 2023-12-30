use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct JoypadInputRequest {
    pub Start: bool,
    pub Select: bool,
    pub B: bool,
    pub A: bool,
    pub Down: bool,
    pub Up: bool,
    pub Left: bool,
    pub Right: bool,
}

impl JoypadInputRequest {
    pub fn new() -> JoypadInputRequest {
        JoypadInputRequest::default()
    }
}

enum ButtonSelector {
    StartSelectBA,
    DownUpLeftRight,
    None,
}

pub struct Joypad {
    need_interrupt: bool,
    buttons: Arc<RwLock<JoypadInputRequest>>,
    button_selector: ButtonSelector,
}

impl Joypad {
    pub fn new(buttons: Arc<RwLock<JoypadInputRequest>>) -> Joypad {
        Joypad {
            need_interrupt: false,
            buttons,
            button_selector: ButtonSelector::None,
        }
    }

    pub fn set_p1_button_selector(&mut self, value: u8) {
        let button_selector = (value >> 4) & 0b11;
        match button_selector {
            0b11 => self.button_selector = ButtonSelector::None,
            0b01 => self.button_selector = ButtonSelector::StartSelectBA,
            0b10 => self.button_selector = ButtonSelector::DownUpLeftRight,
            _ => panic!("Button selector should never be both on"),
        };
    }

    pub fn get_p1(&self) -> u8 {
        match self.button_selector {
            ButtonSelector::None => 0x3F,
            ButtonSelector::DownUpLeftRight => {
                let mut out = !0b0001_0000;
                let buttons = self.buttons.read().expect("Failed read lock of buttons");
                if buttons.Down {
                    out &= !0b1000;
                }
                if buttons.Up {
                    out &= !0b0100;
                }
                if buttons.Left {
                    out &= !0b0010;
                }
                if buttons.Right {
                    out &= !0b0001;
                }
                out
            }
            ButtonSelector::StartSelectBA => {
                let mut out = !0b0010_0000;
                let buttons = self.buttons.read().expect("Failed read lock of buttons");
                if buttons.Start {
                    out &= !0b1000;
                }
                if buttons.Select {
                    out &= !0b0100;
                }
                if buttons.B {
                    out &= !0b0010;
                }
                if buttons.A {
                    out &= !0b0001;
                }
                out
            }
            _ => panic!("Illegal button selector"),
        }
    }

    pub fn consume_interrupt(&mut self) -> bool {
        let need_interrupt = self.need_interrupt;
        self.need_interrupt = false;
        need_interrupt
    }
}
