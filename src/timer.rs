use crate::conf::*;
use crate::util::*;

pub struct Timer {
    div: u8,
    tac: u8,
    tma: u8,
    tima: u8,
    div_ticker: u32,
    tima_ticker: u32,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            div: 0,
            tac: 0,
            tma: 0,
            tima: 0,
            div_ticker: 0,
            tima_ticker: 0,
        }
    }

    pub fn tick(&mut self, cpu_cycles: u8) {
        self.div_ticker += cpu_cycles as u32;
        self.tima_ticker += cpu_cycles as u32;
    }

    #[must_use]
    pub fn handle_ticks(&mut self, pre_exec_tma: u8) -> Result<bool, Error> {
        let mut needs_tima_interrupt = false;

        if self.div_ticker >= DIV_REG_UPDATE_PER_MCYCLE {
            self.div_ticker -= DIV_REG_UPDATE_PER_MCYCLE;
            self.div = self.div.wrapping_add(1);
        }

        let (tima_enabled, tima_freq) = self.tac_expand();
        if tima_enabled {
            if self.tima_ticker >= tima_freq {
                self.tima_ticker -= tima_freq;

                if self.tima == u8::MAX {
                    self.tima = pre_exec_tma;

                    needs_tima_interrupt = true;
                } else {
                    self.tima += 1;
                }
            }
        }

        Ok(needs_tima_interrupt)
    }

    pub fn div(&self) -> u8 {
        self.div
    }
    pub fn tac(&self) -> u8 {
        self.tac
    }
    pub fn tma(&self) -> u8 {
        self.tma
    }
    pub fn tima(&self) -> u8 {
        self.tima
    }

    pub fn set_div(&mut self) {
        self.div = 0;
        self.tima = self.tma;
    }
    pub fn set_tac(&mut self, byte: u8) {
        self.tac = byte;
    }
    pub fn set_tma(&mut self, byte: u8) {
        self.tma = byte;
    }
    pub fn set_tima(&mut self, byte: u8) {
        self.tima = byte;
    }

    fn tac_expand(&self) -> (bool, u32) {
        let tima_enabled = is_bit(self.tac, 2);
        let tima_freq = TIMA_UPDATE_PER_MCYCLE[(self.tac & 0b11) as usize];

        (tima_enabled, tima_freq)
    }
}
