use crate::conf::*;

pub struct Timer {
    pub div: u8,
    pub tac: u8,
    tima: u8,
    div_ticker: u16,
    tima_ticker: u32,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            div: 0,
            tac: 0,
            tima: 0,
            div_ticker: 0,
            tima_ticker: 0,
        }
    }

    pub fn tick(&mut self, add: u8) {
        self.div_ticker += add as u16;
        self.tima_ticker += add as u32;
    }

    pub fn handle_ticks(&mut self, old_tac: u8) -> Result<(), Error> {
        if self.div_ticker >= DIV_REG_UPDATE_PER_MCYCLE {
            self.div_ticker -= DIV_REG_UPDATE_PER_MCYCLE;
            self.div = self.div.wrapping_add(1);
        }

        let (timer_enable, timer_clock) = self.tac_expand()?;
        if timer_enable {
            if self.tima_ticker >= timer_clock {
                self.tima_ticker -= timer_clock;

                if self.tima == u8::MAX {
                    self.tima = old_tac;
                    unimplemented!("TIMA interrupt not implemented")
                } else {
                    self.tima = self.tima.wrapping_add(1);
                }
            }
        }

        Ok(())
    }

    fn tac_expand(&self) -> Result<(bool, u32), Error> {
        let timer_enable = (self.tac & 0b100) > 0;
        let timer_clock = TIMA_UPDATE_PER_MCYCLE[(self.tac & 0b11) as usize];

        Ok((timer_enable, timer_clock))
    }
}
