use crate::conf::*;

pub struct Gfx {
    vram: Vram,
    oam_ram: OamVram,
}

impl Gfx {
    pub fn new(vram: Vram, oam_ram: OamVram) -> Self {
        Gfx { vram, oam_ram }
    }

    pub fn run(&self) {}
}
