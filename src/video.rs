use crate::conf::*;
use crate::util::*;
use std::thread;
use std::time::Duration;
use std::time::Instant;

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

pub const VIDEO_RESULT_MASK_STAT_INTERRUPT: u8 = 0b1;
pub const VIDEO_RESULT_MASK_VBLANK_INTERRUPT: u8 = 0b10;

pub struct Video {
    pub stat_counter: u64,
    // Used to know the variable len of an M3 phase, so M0 can be adjusted.
    prev_m3_len: u64,
    lcdc: u8,
    stat: u8,
    scy: u8,
    scx: u8,
    pub ly: u8,
    lyc: u8,
    bgp: u8,
    obp0: u8,
    obp1: u8,
    wy: u8,
    wx: u8,
    fps_ctrl_time: Instant,
    vram: [u8; VRAM_SIZE],
    oam_ram: [u8; OAM_RAM_SIZE],
    display_buffer: [u8; DISPLAY_PIXELS_COUNT],
    ignore_fps_limiter: bool,
}

impl Video {
    pub fn new(ignore_fps_limiter: bool) -> Self {
        Video {
            stat_counter: 0,
            prev_m3_len: 204,
            lcdc: 0,
            stat: 0x80,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            bgp: 0,
            obp0: 0,
            obp1: 0,
            wy: 0,
            wx: 0,
            fps_ctrl_time: Instant::now(),
            vram: [0; VRAM_SIZE],
            oam_ram: [0; OAM_RAM_SIZE],
            display_buffer: [0; DISPLAY_PIXELS_COUNT],
            ignore_fps_limiter,
        }
    }

    pub fn reset(&mut self) {
        // Bit-7: Should be unused, not sure why BGB has it set.
        // Bit-2: LYC == LY (Read-only): Set when LY contains the same value as LYC; it is constantly updated.
        self.stat = 0b1000_0110;
        self.ly = 0;
    }

    /**
     * Return: interrupt mask.
     */
    // TODO: Use stat interrupt:
    //       - leverage https://gbdev.io/pandocs/Interrupt_Sources.html#int-48--stat-interrupt
    //       - use is_stat_mode_0_interrupt_selected ...
    //       - make return a list of instructions
    #[must_use]
    pub fn update(&mut self, cpu_cycles: u64) -> u8 {
        let mut interrupt_mask = 0;
        if !self.is_lcd_display_enabled() {
            return interrupt_mask;
        }

        self.stat_counter += cpu_cycles;

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
                let m3_len = 204u64;
                if self.stat_counter >= m3_len {
                    self.stat_counter -= m3_len;

                    self.prev_m3_len = m3_len;

                    self.draw_line_to_screen(self.ly);

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
                    self.update_ly(self.ly + 1, &mut interrupt_mask);

                    if self.ly < 144 {
                        // Mode to 2.
                        self.set_lcd_stat_ppu_mode(2);
                    } else {
                        // Mode to 1.
                        self.set_lcd_stat_ppu_mode(1);
                        interrupt_mask |= VIDEO_RESULT_MASK_VBLANK_INTERRUPT;
                    }
                }
            }
            // Waiting until the next frame.
            LcdPpuMode::M1 => {
                if self.stat_counter >= 4560 {
                    self.stat_counter -= 4560;

                    self.update_ly(0, &mut interrupt_mask);

                    // Mode to 2.
                    self.set_lcd_stat_ppu_mode(2);

                    self.ensure_fps();
                } else {
                    self.update_ly(144 + (self.stat_counter / 456) as u8, &mut interrupt_mask);
                }
            }
        };

        interrupt_mask
    }

    fn update_ly(&mut self, new_ly: u8, interrupt_mask: &mut u8) {
        self.ly = new_ly;

        if self.ly == self.lyc {
            self.stat |= 0b0100;

            if self.is_lyc_coincidence_interrupt_enabled() {
                *interrupt_mask = *interrupt_mask | VIDEO_RESULT_MASK_STAT_INTERRUPT;
            }
        } else {
            self.stat &= 0b1111_1011;
        }
    }

    pub fn draw_line_to_screen(&mut self, ly: u8) {
        if self.is_window_display_enabled() {
            self.draw_window_to_screen(ly);
        }

        if self.is_background_window_display_priority() {
            self.draw_background_map_to_screen(ly);
        }

        if self.is_obj_sprite_display_enabled() {
            self.draw_objects_to_screen(ly);
        }
    }

    fn draw_window_to_screen(&mut self, ly: u8) {
        // NOT IMPLEMENTED
    }

    fn draw_objects_to_screen(&mut self, ly: u8) {
        // NOT IMPLEMENTED
    }

    fn draw_background_map_to_screen(&mut self, ly: u8) {
        let tile_map_start = self.background_tile_map_display_section_start();
        // There are 32x32 tiles on the map: 256x256 pixels

        let actual_ly = ly.wrapping_add(self.scy);

        let tile_row = actual_ly / 8;
        let tile_y = actual_ly % 8;

        for i in 0..DISPLAY_WIDTH {
            let actual_x = self.scx.wrapping_add(i as u8);
            let tile_col = actual_x / 8;
            let tile_x = (actual_x % 8) as u8;
            let tile_data_i = (tile_row as u16 * 32) + tile_col as u16;
            let tile_i = self
                .read(tile_map_start + tile_data_i as u16)
                .expect("Failed getting tile data");
            let tile_lo = self
                .read(
                    self.backround_window_tile_data_section_start()
                        + tile_i as u16 * 16
                        + tile_y as u16 * 2,
                )
                .expect("Cannot load bg tile");
            let tile_hi = self
                .read(
                    self.backround_window_tile_data_section_start()
                        + tile_i as u16 * 16
                        + tile_y as u16 * 2
                        + 1,
                )
                .expect("Cannot load bg tile");
            let color = (bit(tile_hi, 7 - tile_x) << 1) | bit(tile_lo, 7 - tile_x);

            self.display_buffer[ly as usize * DISPLAY_WIDTH as usize + i as usize] = color;
        }
    }

    pub fn read(&self, loc: u16) -> Result<u8, Error> {
        let byte = match loc {
            MEM_AREA_VRAM_START..=MEM_AREA_VRAM_END => {
                self.vram[(loc - MEM_AREA_VRAM_START) as usize]
            }
            MEM_AREA_OAM_START..=MEM_AREA_OAM_END => {
                self.oam_ram[(loc - MEM_AREA_OAM_START) as usize]
            }
            MEM_LOC_LCDC => self.lcdc,
            MEM_LOC_STAT => self.stat,
            MEM_LOC_SCY => self.scy,
            MEM_LOC_SCX => self.scx,
            MEM_LOC_LY => self.ly,
            MEM_LOC_LYC => self.lyc,
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
            MEM_AREA_VRAM_START..=MEM_AREA_VRAM_END => {
                self.vram[(loc - MEM_AREA_VRAM_START) as usize] = byte;
            }
            MEM_AREA_OAM_START..=MEM_AREA_OAM_END => {
                self.oam_ram[(loc - MEM_AREA_OAM_START) as usize] = byte;
            }
            MEM_LOC_LCDC => self.lcdc = byte,
            MEM_LOC_STAT => {
                self.stat = byte | 0x80;
                // These 3 bytes are the stat interrupt enable bytes. We do not handle them on PPU  mode change.
                assert!((byte & 0b0011_1000) == 0);
            }
            MEM_LOC_SCY => self.scy = byte,
            MEM_LOC_SCX => self.scx = byte,
            MEM_LOC_LY => panic!("Cannot write LY"),
            MEM_LOC_LYC => {
                self.lyc = byte;
                // TODO: This probably needs an LY==LYC interrupt check.
            }
            MEM_LOC_BGP => self.bgp = byte,
            MEM_LOC_OBP0 => self.obp0 = byte,
            MEM_LOC_OBP1 => self.obp1 = byte,
            MEM_LOC_WY => self.wy = byte,
            MEM_LOC_WX => self.wx = byte,
            _ => panic!("Illegal video address write: {:#06X}", loc),
        }
    }

    pub fn dma_oam_transfer(&mut self, block: Vec<u8>) {
        assert!(block.len() == 0xA0);

        for (i, byte) in block.iter().enumerate() {
            self.write(MEM_AREA_OAM_START + i as u16, *byte);
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

    fn window_tile_map_display_section_start(&self) -> u16 {
        match self.window_tile_map_display_section() {
            TileMapDisplaySelect::Section9800_9BFF => 0x9800,
            TileMapDisplaySelect::Section9C00_9FFF => 0x9C00,
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

    fn backround_window_tile_data_section_start(&self) -> u16 {
        match self.backround_window_tile_data_section() {
            BgrWinTileDataSelect::Section8000_8FFF => 0x8000,
            BgrWinTileDataSelect::Section8800_97FF => 0x8800,
        }
    }

    fn background_tile_map_display_section(&self) -> TileMapDisplaySelect {
        if is_bit(self.lcdc, 3) {
            TileMapDisplaySelect::Section9C00_9FFF
        } else {
            TileMapDisplaySelect::Section9800_9BFF
        }
    }

    fn background_tile_map_display_section_start(&self) -> u16 {
        match self.background_tile_map_display_section() {
            TileMapDisplaySelect::Section9800_9BFF => 0x9800,
            TileMapDisplaySelect::Section9C00_9FFF => 0x9C00,
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

    fn ensure_fps(&mut self) {
        if self.ignore_fps_limiter {
            return;
        }

        let elapsed = self.fps_ctrl_time.elapsed().as_micros();
        if elapsed < ONE_FRAME_IN_MICROSECONDS as u128 {
            thread::sleep(Duration::from_micros(
                ONE_FRAME_IN_MICROSECONDS as u64 - elapsed as u64,
            ));
        }

        self.fps_ctrl_time = Instant::now();
    }

    /**
     * Expect a 16 tile wide 3 x 8 tile tall grid:
     * Pixel width:  16 (tile) * 8 (pixel per tile)
     * Pixel height: 24 (tile) * 8 (pixel per tile)
     * -> Total: 16 * 8 * 24 * 8 * 4 (color bytes per pixel)
     */
    pub fn draw_debug_tiles(&self, frame: &mut [u8]) {
        const FRAME_LINE_OFFS: usize = 16 * 8 * 4;

        for y in 0..24 {
            for x in 0..16 {
                let tile_number = (y * 16) + x;
                let vram_pos = tile_number * 16; // 8x8 pixel with 2bpp = 16 bytes
                let frame_pos = (y * 8 * 8 * 4 * 16) + (x * 8 * 4); // Assuming frame is 4-attr color (RGBA) * 8x8 sprite size
                for sprite_y in 0..8 {
                    let byte1 = self.vram[vram_pos + sprite_y * 2];
                    let byte2 = self.vram[vram_pos + sprite_y * 2 + 1];
                    for sprite_x in 0..8 {
                        let gb_pixel_color = (((byte2 >> (7 - sprite_x)) & 0b1) << 1)
                            | ((byte1 >> (7 - sprite_x)) & 0b1);

                        let pixel_color = self.pixel_color(gb_pixel_color);

                        let frame_pos_pixel_offset = sprite_x * 4;
                        frame
                            [frame_pos + FRAME_LINE_OFFS * sprite_y + frame_pos_pixel_offset + 0] =
                            pixel_color[0];
                        frame
                            [frame_pos + FRAME_LINE_OFFS * sprite_y + frame_pos_pixel_offset + 1] =
                            pixel_color[1];
                        frame
                            [frame_pos + FRAME_LINE_OFFS * sprite_y + frame_pos_pixel_offset + 2] =
                            pixel_color[2];
                        frame
                            [frame_pos + FRAME_LINE_OFFS * sprite_y + frame_pos_pixel_offset + 3] =
                            pixel_color[3];
                    }
                }
            }
        }
    }

    pub fn draw_display(&self, frame: &mut [u8]) {
        for y in 0..DISPLAY_HEIGHT {
            for x in 0..DISPLAY_WIDTH {
                let pixel_pos: usize = ((y * DISPLAY_WIDTH) + x) as usize;
                let frame_pos: usize = pixel_pos * 4;
                let pixel_color = self.pixel_color(self.display_buffer[pixel_pos]);

                frame[frame_pos + 0] = pixel_color[0];
                frame[frame_pos + 1] = pixel_color[1];
                frame[frame_pos + 2] = pixel_color[2];
                frame[frame_pos + 3] = pixel_color[3];
            }
        }
    }

    fn pixel_color(&self, code: u8) -> [u8; 4] {
        match code {
            0b11 => [0x10, 0x40, 0x20, 0xff],
            0b10 => [0x10, 0x80, 0x40, 0xff],
            0b01 => [0x10, 0xa0, 0x50, 0xff],
            0b00 => [0x10, 0xf0, 0x80, 0xff],
            _ => unimplemented!("Unknown gb pixel color"),
        }
    }
}
