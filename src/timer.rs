use crate::conf::*;
use crate::util::*;

pub struct Timer {
    div: u8,
    tac: u8,
    tma: u8,
    tima: u8,
    div_ticker: Counter,
    tima_ticker: Counter,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            div: 0,
            // High 5 bytes unused - set to 1.
            tac: 0b1111_1000,
            tma: 0,
            tima: 0,
            div_ticker: Counter::new(DIV_REG_UPDATE_PER_MCYCLE as u64),
            tima_ticker: Counter::new(TIMA_UPDATE_PER_MCYCLE[3] as u64),
        }
    }

    pub fn tick(&mut self, cpu_clocks: u8) {
        self.div_ticker.tick(cpu_clocks as u64);

        let tima_enabled = is_bit(self.tac, 2);
        if tima_enabled {
            self.tima_ticker.tick(cpu_clocks as u64);
        }
    }

    #[must_use]
    pub fn handle_ticks(&mut self, pre_exec_tma: u8) -> Result<bool, Error> {
        // println!("DIV: {} TIMA: {}", self.div, self.tima);

        let mut needs_tima_interrupt = false;

        if self.div_ticker.check_overflow() {
            self.div = self.div.wrapping_add(1);
        }

        let tima_enabled = is_bit(self.tac, 2);
        if tima_enabled {
            let tima_freq = TIMA_UPDATE_PER_MCYCLE[(self.tac & 0b11) as usize];
            self.tima_ticker.update_modulo(tima_freq as u64);

            let mut overflow_count = self.tima_ticker.check_overflow_count();
            while overflow_count > 0 {
                if self.tima == 0xFF {
                    self.tima = pre_exec_tma;

                    needs_tima_interrupt = true;
                } else {
                    self.tima += 1;
                }

                overflow_count -= 1;
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
    }
    pub fn set_tac(&mut self, byte: u8) {
        self.tac = byte | 0b1111_1000; // Keep useless bytes to 1.
    }
    pub fn set_tma(&mut self, byte: u8) {
        self.tma = byte;
    }
    pub fn set_tima(&mut self, byte: u8) {
        self.tima = byte;
    }
}
