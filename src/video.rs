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
    stat_counter: u64,
    // Used to know the variable len of an M3 phase, so M0 can be adjusted.
    prev_m3_len: u64,
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
            stat_counter: 0,
            prev_m3_len: 0,
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

    pub fn reset(&mut self) {
        // Set LCD stat mode to 2.
        self.lcdc = (self.lcdc & 0b1111_1100) | 0b10;
        self.ly = 0;
    }

    pub fn update(&mut self, spent_mcycle: u64) {
        if !self.is_lcd_display_enabled() {
            return;
        }

        self.stat_counter += spent_mcycle;

        // Mode 2  2_____2_____2_____2_____2_____2___________________2____
        // Mode 3  _33____33____33____33____33____33__________________3___
        // Mode 0  ___000___000___000___000___000___000________________000
        // Mode 1  ____________________________________11111111111111_____
        match self.lcdc_mode() {
            LcdcMode::M2 => {
                if self.stat_counter >= 80 {
                    self.stat_counter -= 80;
                    // Mode to 3.
                    self.lcdc = (self.lcdc & 0b1111_1100) | 0b11;
                }
            }
            LcdcMode::M3 => {
                // Todo: 168 to 291 dots, depending on object count
                let m3_len = 200u64;
                if self.stat_counter >= m3_len {
                    self.stat_counter -= m3_len;

                    self.prev_m3_len = m3_len;

                    // Todo: draw for line LY.

                    // Mode to 0.
                    self.lcdc = self.lcdc & 0b1111_1100;
                }
            }
            LcdcMode::M0 => {
                let m0_len = 85 + 291 - 168 - self.prev_m3_len;

                if self.stat_counter >= m0_len {
                    self.stat_counter -= m0_len;

                    // Increase LY.
                    self.ly += 1;
                    if self.ly == self.lyc {
                        unimplemented!("LYC STAT INT");
                    }

                    if self.ly < 144 {
                        // Mode to 2.
                        self.lcdc = (self.lcdc & 0b1111_1100) | 0b10;
                    } else {
                        // Mode to 1.
                        self.lcdc = (self.lcdc & 0b1111_1100) | 0b1;
                    }
                }
            }
            LcdcMode::M1 => {
                if self.stat_counter >= 4560 {
                    self.stat_counter -= 4560;

                    self.ly = 0;

                    // Mode to 2.
                    self.lcdc = (self.lcdc & 0b1111_1100) | 0b10;
                } else {
                    self.ly = 144 + (self.stat_counter / 506) as u8;
                }
            }
        };

        // Update LY
        // Update LYC
        // Handle draw stages/modes
    }

    pub fn read(&self, loc: u16) -> Result<u8, Error> {
        let byte = match loc {
            MEM_LOC_LCDC => self.lcdc,
            MEM_LOC_STAT => self.stat,
            MEM_LOC_SCY => self.scy,
            MEM_LOC_SCX => self.scx,
            MEM_LOC_LY => self.ly,
            MEM_LOC_LYC => self.lyc,
            MEM_LOC_DMA => self.dma,
            MEM_LOC_BGP => self.bgp,
            MEM_LOC_OBP0 => self.obp0,
            MEM_LOC_OBP1 => self.obp1,
            MEM_LOC_WY => self.wy,
            MEM_LOC_WX => self.wx,
            _ => panic!("Illegal video address read: {:#06X}", loc),
        };

        log::debug!("Read video: {:#06X} = #{:#04X}", loc, byte);

        Ok(byte)
    }

    pub fn write(&mut self, loc: u16, byte: u8) {
        log::debug!("Write video: {:#06X} = #{:#04X}", loc, byte);

        match loc {
            MEM_LOC_LCDC => self.lcdc = byte,
            MEM_LOC_STAT => self.stat = byte,
            MEM_LOC_SCY => self.scy = byte,
            MEM_LOC_SCX => self.scx = byte,
            MEM_LOC_LY => self.ly = byte,
            MEM_LOC_LYC => self.lyc = byte,
            MEM_LOC_DMA => {
                unimplemented!("Unimplemented DMA write. This starts an OAM copy.");
                self.dma = byte;
            }
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
