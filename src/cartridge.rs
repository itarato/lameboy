use crate::conf::*;

pub struct Cartridge {}

impl Cartridge {
    pub fn new() -> Result<Self, Error> {
        Ok(Cartridge {})
    }

    pub fn rom0(&self) -> &[u8] {
        unimplemented!()
    }
}
