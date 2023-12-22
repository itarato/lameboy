use crate::conf::*;
use crate::util::*;

pub struct Timer {
    pub div: u8,
    pub tac: u8,
    pub tma: u8,
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

    pub fn tick(&mut self, add: u8) {
        self.div_ticker += add as u32;
        self.tima_ticker += add as u32;
    }

    pub fn handle_ticks(&mut self, pre_exec_tma: u8) -> Result<(), Error> {
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

                    unimplemented!("TIMA interrupt not implemented")
                } else {
                    self.tima += 1;
                }
            }
        }

        Ok(())
    }

    fn tac_expand(&self) -> (bool, u32) {
        let tima_enabled = is_bit(self.tac, 2);
        let tima_freq = TIMA_UPDATE_PER_MCYCLE[(self.tac & 0b11) as usize];

        (tima_enabled, tima_freq)
    }
}
