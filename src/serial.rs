use log::warn;

pub struct Serial;

impl Serial {
    pub fn new() -> Serial {
        Serial
    }

    pub fn set_sb(&mut self, value: u8) {
        warn!("Serial - SB Set: {}", value);
    }

    pub fn set_sc(&mut self, value: u8) {
        warn!("Serial - SC Set: {}", value);
    }
}
