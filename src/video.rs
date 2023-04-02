use crate::conf::*;
use crate::util::*;

enum LcdcMode {
    M0,
    M1,
    M2,
    M3,
}

enum TileMapDisplaySelect {
    Section9800_9BFF,
    Section9C00_9FFF,
}

enum BgrWinTileDataSelect {
    Section8800_97FF,
    Section8000_8FFF,
}

enum ObjSpriteSize {
    Size8x8,
    Size8x16,
}

enum Coincidence {
    LycIsNotLy,
    LyxIsLy,
}

pub struct Video {
    lcdc: u8,
    stat: u8,
    scy: u8,
    scx: u8,
    ly: u8,
    lyc: u8,
    dma: u8,
    bgp: u8,
    obp0: u8,
    obp1: u8,
    wy: u8,
    wx: u8,
}

impl Video {
    pub fn new() -> Self {
        Video {
            lcdc: 0,
            stat: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            dma: 0,
            bgp: 0,
            obp0: 0,
            obp1: 0,
            wy: 0,
            wx: 0,
        }
    }

    pub fn write(&mut self, loc: u16, byte: u8) {
        match loc {
            MEM_LOC_LCDC => self.lcdc = byte,
            MEM_LOC_STAT => self.stat = byte,
            MEM_LOC_SCY => self.scy = byte,
            MEM_LOC_SCX => self.scx = byte,
            MEM_LOC_LY => self.ly = byte,
            MEM_LOC_LYC => self.lyc = byte,
            MEM_LOC_DMA => self.dma = byte,
            MEM_LOC_BGP => self.bgp = byte,
            MEM_LOC_OBP0 => self.obp0 = byte,
            MEM_LOC_OBP1 => self.obp1 = byte,
            MEM_LOC_WY => self.wy = byte,
            MEM_LOC_WX => self.wx = byte,
            _ => panic!("Illegal video address write: {:#06X}", loc),
        }
    }

    fn is_lcd_display_enabled(&self) -> bool {
        is_bit(self.lcdc, 7)
    }

    fn window_tile_map_display_section(&self) -> TileMapDisplaySelect {
        if is_bit(self.lcdc, 6) {
            TileMapDisplaySelect::Section9C00_9FFF
        } else {
            TileMapDisplaySelect::Section9800_9BFF
        }
    }

    fn is_window_display_enabled(&self) -> bool {
        is_bit(self.lcdc, 5)
    }

    fn backround_window_tile_data_section(&self) -> BgrWinTileDataSelect {
        if is_bit(self.lcdc, 4) {
            BgrWinTileDataSelect::Section8000_8FFF
        } else {
            BgrWinTileDataSelect::Section8800_97FF
        }
    }

    fn background_tile_map_display_section(&self) -> TileMapDisplaySelect {
        if is_bit(self.lcdc, 3) {
            TileMapDisplaySelect::Section9C00_9FFF
        } else {
            TileMapDisplaySelect::Section9800_9BFF
        }
    }

    fn obj_sprite_size(&self) -> ObjSpriteSize {
        if is_bit(self.lcdc, 2) {
            ObjSpriteSize::Size8x16
        } else {
            ObjSpriteSize::Size8x8
        }
    }

    fn is_obj_sprite_display_enabled(&self) -> bool {
        is_bit(self.lcdc, 1)
    }

    fn is_background_window_display_priority(&self) -> bool {
        is_bit(self.lcdc, 0)
    }

    fn is_lyc_coincidence_interrupt_enabled(&self) -> bool {
        is_bit(self.stat, 6)
    }

    fn is_mode2_oam_interrupt_enabled(&self) -> bool {
        is_bit(self.stat, 5)
    }

    fn is_mode1_vblank_interrupt_enabled(&self) -> bool {
        is_bit(self.stat, 4)
    }

    fn is_mode0_hblank_interrupt_enabled(&self) -> bool {
        is_bit(self.stat, 3)
    }

    fn coincidence_flag(&self) -> Coincidence {
        if is_bit(self.stat, 2) {
            Coincidence::LyxIsLy
        } else {
            Coincidence::LycIsNotLy
        }
    }

    fn lcdc_mode(&self) -> LcdcMode {
        match self.stat & 0b11 {
            0b00 => LcdcMode::M0,
            0b01 => LcdcMode::M1,
            0b10 => LcdcMode::M2,
            0b11 => LcdcMode::M3,
            _ => panic!("Illegal LCDC mode"),
        }
    }
}
