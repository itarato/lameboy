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
            div_ticker: Counter::new(DIV_REG_UPDATE_PER_MCYCLE),
            tima_ticker: Counter::new(TIMA_UPDATE_PER_MCYCLE[3]),
        }
    }

    #[must_use]
    pub fn handle_ticks(&mut self, cpu_clocks: u32, pre_exec_tma: u8) -> Result<bool, Error> {
        // println!("DIV: {} TIMA: {}", self.div, self.tima);

        let mut needs_tima_interrupt = false;

        self.div_ticker.tick(cpu_clocks as _);
        if self.div_ticker.check_overflow() {
            self.div = self.div.wrapping_add(1);
        }

        let tima_enabled = is_bit(self.tac, 2);
        if tima_enabled {
            self.tima_ticker.tick(cpu_clocks as _);

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
        self.div_ticker.reset();
    }
    pub fn set_tac(&mut self, byte: u8) {
        if byte & 0b11 != self.tac & 0b11 {
            self.tima_ticker.reset();
        }

        self.tac = byte | 0b1111_1000; // Keep useless bytes to 1.

        let tima_freq = TIMA_UPDATE_PER_MCYCLE[(self.tac & 0b11) as usize];
        self.tima_ticker.update_modulo(tima_freq);
    }
    pub fn set_tma(&mut self, byte: u8) {
        self.tma = byte;
    }
    pub fn set_tima(&mut self, byte: u8) {
        self.tima = byte;
    }

    pub fn dump_debug_panel(&self) {
        println!("\x1B[93mDIV\x1B[0m {:02X} | \x1B[93mTIMA\x1B[0m {:02X} ({:X}) | \x1B[93mTMA\x1B[0m {:02X} | \x1B[93mTAC\x1B[0m {:02X}", self.div, self.tima, self.tima_ticker.counter, self.tma, self.tac);
    }
}
