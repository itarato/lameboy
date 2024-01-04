use crate::conf::Error;
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct JoypadInputRequest {
    pub start: bool,
    pub select: bool,
    pub b: bool,
    pub a: bool,
    pub down: bool,
    pub up: bool,
    pub left: bool,
    pub right: bool,
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

    pub fn set_p1_button_selector(&mut self, value: u8) -> Result<(), Error> {
        let button_selector = (value >> 4) & 0b11;
        match button_selector {
            0b11 | 0b00 => self.button_selector = ButtonSelector::None,
            0b01 => self.button_selector = ButtonSelector::StartSelectBA,
            0b10 => self.button_selector = ButtonSelector::DownUpLeftRight,
            _ => {
                return Err(format!(
                    "Invalid P1, button selector should never be both on: 0b{:08b}",
                    value
                )
                .into())
            }
        };

        Ok(())
    }

    pub fn get_p1(&self) -> u8 {
        match self.button_selector {
            ButtonSelector::None => 0x3F,
            ButtonSelector::DownUpLeftRight => {
                let mut out = !0b0001_0000;
                let buttons = self.buttons.read().expect("Failed read lock of buttons");
                if buttons.down {
                    out &= !0b1000;
                }
                if buttons.up {
                    out &= !0b0100;
                }
                if buttons.left {
                    out &= !0b0010;
                }
                if buttons.right {
                    out &= !0b0001;
                }
                out
            }
            ButtonSelector::StartSelectBA => {
                let mut out = !0b0010_0000;
                let buttons = self.buttons.read().expect("Failed read lock of buttons");
                if buttons.start {
                    out &= !0b1000;
                }
                if buttons.select {
                    out &= !0b0100;
                }
                if buttons.b {
                    out &= !0b0010;
                }
                if buttons.a {
                    out &= !0b0001;
                }
                out
            }
        }
    }

    pub fn consume_interrupt(&mut self) -> bool {
        let need_interrupt = self.need_interrupt;
        self.need_interrupt = false;
        need_interrupt
    }
}
