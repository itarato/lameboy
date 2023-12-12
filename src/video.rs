use crate::conf::*;
use crate::util::*;

enum LcdPpuMode {
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
    pub ly: u8,
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
        // Bit-7: Should be unused, not sure why BGB has it set.
        // Bit-2: LYC == LY (Read-only): Set when LY contains the same value as LYC; it is constantly updated.
        self.stat = 0b1000_0100;

        self.ly = 0;
    }

    /**
     * Return: whether vblank interrupt should be called.
     */
    // TODO: Use stat interrupt:
    //       - leverage https://gbdev.io/pandocs/Interrupt_Sources.html#int-48--stat-interrupt
    //       - use is_stat_mode_0_interrupt_selected ...
    //       - make return a list of instructions
    #[must_use]
    pub fn update(&mut self, spent_mcycle: u64) -> bool {
        let mut should_call_vblank_interrupt = false;
        if !self.is_lcd_display_enabled() {
            return should_call_vblank_interrupt;
        }

        self.stat_counter += spent_mcycle;

        // Mode 2  2_____2_____2_____2_____2_____2___________________2____
        // Mode 3  _33____33____33____33____33____33__________________3___
        // Mode 0  ___000___000___000___000___000___000________________000
        // Mode 1  ____________________________________11111111111111_____
        match self.lcd_ppu_mode() {
            // Searching for OBJs which overlap this line.
            LcdPpuMode::M2 => {
                if self.stat_counter >= 80 {
                    self.stat_counter -= 80;
                    // Mode to 3.
                    self.set_lcd_stat_ppu_mode(3);
                }
            }
            // Sending pixels to the LCD.
            LcdPpuMode::M3 => {
                // Todo: 172 to 289 dots, depending on object count
                let m3_len = 200u64;
                if self.stat_counter >= m3_len {
                    self.stat_counter -= m3_len;

                    self.prev_m3_len = m3_len;

                    // Todo: draw for line LY.

                    // Mode to 0.
                    self.set_lcd_stat_ppu_mode(0);
                }
            }
            // Waiting until the end of the scanline.
            LcdPpuMode::M0 => {
                // Todo: 87 to 204 dots (I assume depending on object count, reverse (vs M3))
                let m0_len = 87 + 289 - self.prev_m3_len;

                if self.stat_counter >= m0_len {
                    self.stat_counter -= m0_len;

                    // Increase LY.
                    self.ly += 1;
                    if self.ly == self.lyc {
                        unimplemented!("LYC STAT INT");
                    }

                    if self.ly < 144 {
                        // Mode to 2.
                        self.set_lcd_stat_ppu_mode(2);
                    } else {
                        // Mode to 1.
                        self.set_lcd_stat_ppu_mode(1);
                        should_call_vblank_interrupt = true;
                    }
                }
            }
            // Waiting until the next frame.
            LcdPpuMode::M1 => {
                if self.stat_counter >= 4560 {
                    self.stat_counter -= 4560;

                    self.ly = 0;

                    // Mode to 2.
                    self.set_lcd_stat_ppu_mode(2);
                } else {
                    self.ly = 144 + (self.stat_counter / 506) as u8;
                }
            }
        };

        // Update LY
        // Update LYC
        if self.ly == self.lyc {
            self.stat |= 0b0100;
        } else {
            self.stat &= !0b0100;
        }
        // Handle draw stages/modes

        should_call_vblank_interrupt
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
            MEM_LOC_STAT => {
                self.stat = byte;
                // These 3 bytes are the stat interrupt enable bytes. We do not handle them on PPU  mode change.
                assert!((byte & 0b0011_1000) == 0);
            }
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

    fn lcd_ppu_mode(&self) -> LcdPpuMode {
        match self.stat & 0b11 {
            0b00 => LcdPpuMode::M0,
            0b01 => LcdPpuMode::M1,
            0b10 => LcdPpuMode::M2,
            0b11 => LcdPpuMode::M3,
            _ => panic!("Illegal LCDC mode"),
        }
    }

    fn set_lcd_stat_ppu_mode(&mut self, mode: u8) {
        assert!(mode <= 0b11);
        self.stat &= 0b1111_1100;
        self.stat |= mode;
    }

    fn is_stat_mode_0_interrupt_selected(&self) -> bool {
        (self.stat & 0b1000) > 0
    }

    fn is_stat_mode_1_interrupt_selected(&self) -> bool {
        (self.stat & 0b1_0000) > 0
    }

    fn is_stat_mode_2_interrupt_selected(&self) -> bool {
        (self.stat & 0b10_0000) > 0
    }
}
