use winit::window::WindowId;

use crate::conf::*;
use crate::util::*;
use std::thread;
use std::time::Duration;
use std::time::Instant;

#[derive(PartialEq)]
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

pub struct PPU {
    pub stat_counter: u64,
    // Used to know the variable len of an M3 phase, so M0 can be adjusted.
    prev_m3_len: u64,
    pub lcdc: u8,
    pub stat: u8,
    scy: u8,
    scx: u8,
    pub ly: u8,
    pub lyc: u8,
    bgp: u8,
    obp0: u8,
    obp1: u8,
    pub wy: u8,
    pub wx: u8,
    fps_ctrl_time: Instant,
    vram: [u8; VRAM_SIZE],
    oam_ram: [u8; OAM_RAM_SIZE],
    display_buffer: [u8; DISPLAY_PIXELS_COUNT << 2],
    ignore_fps_limiter: bool,
    pub main_window_id: Option<WindowId>,
    pub tile_debug_window_id: Option<WindowId>,
    pub background_debug_window_id: Option<WindowId>,
    pub window_debug_window_id: Option<WindowId>,
    lyc_change_interrupt: bool,
    wy_offset: u8,
}

impl PPU {
    pub fn new(ignore_fps_limiter: bool) -> Self {
        PPU {
            stat_counter: 0,
            prev_m3_len: 252,
            lcdc: 0,
            stat: 0x82,
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
            display_buffer: [0; DISPLAY_PIXELS_COUNT << 2],
            ignore_fps_limiter,
            main_window_id: None,
            tile_debug_window_id: None,
            background_debug_window_id: None,
            window_debug_window_id: None,
            lyc_change_interrupt: false,
            wy_offset: 0,
        }
    }

    pub fn reset(&mut self) {
        // Bit-7: Should be unused, not sure why BGB has it set.
        // Bit-2: LYC == LY (Read-only): Set when LY contains the same value as LYC; it is constantly updated.
        self.stat = 0b1000_0110;
        self.ly = 0;
        self.stat_counter = 0;
        self.lcdc = 0;
        self.stat = 0x82;
        self.prev_m3_len = 252;
        self.scy = 0;
        self.scx = 0;
        self.ly = 0;
        self.lyc = 0;
        self.bgp = 0;
        self.obp0 = 0;
        self.obp1 = 0;
        self.wy = 0;
        self.wx = 0;
        self.vram.iter_mut().for_each(|b| *b = 0);
        self.oam_ram.iter_mut().for_each(|b| *b = 0);
        self.display_buffer.iter_mut().for_each(|b| *b = 0);
        self.lyc_change_interrupt = false;
        self.wy_offset = 0;
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

        // println!("Ticks: {}", cpu_cycles);

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
                    if self.set_lcd_stat_ppu_mode(3) {
                        interrupt_mask |= VIDEO_RESULT_MASK_STAT_INTERRUPT;
                    }
                    self.draw_line_to_screen(self.ly);
                }
            }
            // Sending pixels to the LCD.
            LcdPpuMode::M3 => {
                // Todo: 172 to 289 dots, depending on object count
                let m3_len = 252;
                if self.stat_counter >= m3_len {
                    self.stat_counter -= m3_len;

                    self.prev_m3_len = m3_len;

                    // Mode to 0.
                    if self.set_lcd_stat_ppu_mode(0) {
                        interrupt_mask |= VIDEO_RESULT_MASK_STAT_INTERRUPT;
                    }
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
                        if self.set_lcd_stat_ppu_mode(2) {
                            interrupt_mask |= VIDEO_RESULT_MASK_STAT_INTERRUPT;
                        }
                    } else {
                        // Mode to 1.
                        if self.set_lcd_stat_ppu_mode(1) {
                            interrupt_mask |= VIDEO_RESULT_MASK_STAT_INTERRUPT;
                        }
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
                    if self.set_lcd_stat_ppu_mode(2) {
                        interrupt_mask |= VIDEO_RESULT_MASK_STAT_INTERRUPT;
                    }

                    self.ensure_fps();
                } else {
                    self.update_ly(144 + (self.stat_counter / 456) as u8, &mut interrupt_mask);
                }
            }
        };

        if self.lyc_change_interrupt {
            self.lyc_change_interrupt = false;
            interrupt_mask = interrupt_mask | VIDEO_RESULT_MASK_STAT_INTERRUPT;
        }

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
        if self.is_background_window_display_priority() {
            self.draw_background_to_screen(ly);
            self.draw_window_to_screen(ly);
        }

        if self.is_obj_sprite_display_enabled() {
            self.draw_objects_to_screen(ly);
        }
    }

    fn draw_objects_to_screen(&mut self, ly: u8) {
        // Object attributes reside in the object attribute memory (OAM) at $FE00-FE9F.

        // In 8×8 mode (LCDC bit 2 = 0), this byte specifies the object’s only tile index ($00-$FF).
        // This unsigned value selects a tile from the memory area at $8000-$8FFF.
        // In 8×16 mode (LCDC bit 2 = 1), the memory area at $8000-$8FFF is still interpreted as a
        // series of 8×8 tiles, where every 2 tiles form an object.
        let tile_height = match self.obj_sprite_size() {
            ObjSpriteSize::Size8x8 => 8,
            ObjSpriteSize::Size8x16 => 16,
        } as i16;

        for i in 0..40 {
            let byte_y_pos = self.oam_ram[(i * 4) + 0] as i16;
            let mut tile_y = ly as i16 - (byte_y_pos - 16);
            if tile_y < 0 || tile_y >= tile_height {
                // Tile surface does not cover LY.
                continue;
            }

            let byte_x_pos = self.oam_ram[(i * 4) + 1] as i16;
            if byte_x_pos == 0 || byte_x_pos - 8 >= DISPLAY_WIDTH as i16 {
                // Horizontally out of screen.
                continue;
            }

            let byte_tile_index = self.oam_ram[(i * 4) + 2] as usize;
            let byte_attr_and_flags = self.oam_ram[(i * 4) + 3];

            let priority = is_bit(byte_attr_and_flags, 7);
            let y_flip = is_bit(byte_attr_and_flags, 6);
            let x_flip = is_bit(byte_attr_and_flags, 5);
            // DMG palette [Non CGB Mode only]: 0 = OBP0, 1 = OBP1
            let palette = if is_bit(byte_attr_and_flags, 4) {
                self.obp1
            } else {
                self.obp0
            };

            if y_flip {
                tile_y = tile_height - 1 - tile_y;
            }

            // MISSING: 10 sprite per line check.
            // MISSING: mode-3 length adjustment.

            let tile_start_addr = (byte_tile_index * 8 * 2) as usize;

            let row_lo = self.vram[tile_start_addr + (tile_y as usize * 2) + 0];
            let row_hi = self.vram[tile_start_addr + (tile_y as usize * 2) + 1];
            for x in 0..8 {
                let row_bit = if x_flip { x } else { 7 - x };
                let color = (bit(row_hi, row_bit) << 1) | bit(row_lo, row_bit);

                let physical_x = byte_x_pos - 8 + x as i16;
                if physical_x < 0 || physical_x >= DISPLAY_WIDTH as i16 {
                    continue;
                }

                // Transparency check.
                if color == 0 {
                    continue;
                }

                if !priority || self.is_display_pixel_color_zero(physical_x as _, ly as _) {
                    self.set_display_pixel(physical_x as _, ly as _, palette, color);
                }
            }
        }
    }

    // There are 32x32 tiles on the map: 256x256 pixels.
    fn draw_background_to_screen(&mut self, ly: u8) {
        let tile_data_section_start =
            (self.backround_window_tile_data_section_start() - MEM_AREA_VRAM_START) as usize;
        let tile_map_start =
            (self.background_tile_map_display_section_start() - MEM_AREA_VRAM_START) as usize;

        // The background map wraps.
        let actual_ly = ly.wrapping_add(self.scy);

        let tile_row = actual_ly / 8;
        let tile_y = actual_ly % 8;

        for i in 0..DISPLAY_WIDTH {
            let actual_x = self.scx.wrapping_add(i as u8);
            let tile_col = actual_x / 8;
            let tile_x = (actual_x % 8) as u8;
            let tile_data_i = (tile_row as usize * 32) + tile_col as usize;
            let tile_i = self.vram[tile_map_start + tile_data_i];

            let tile_i = if tile_data_section_start == 0x0800 {
                tile_i.wrapping_add(128)
            } else {
                tile_i
            };

            let tile_lo =
                self.vram[tile_data_section_start + tile_i as usize * 16 + tile_y as usize * 2];
            let tile_hi =
                self.vram[tile_data_section_start + tile_i as usize * 16 + tile_y as usize * 2 + 1];
            let color = (bit(tile_hi, 7 - tile_x) << 1) | bit(tile_lo, 7 - tile_x);

            self.set_display_pixel(i as _, ly as _, self.bgp, color);
        }
    }

    fn draw_window_to_screen(&mut self, ly: u8) {
        if ly < self.wy {
            return;
        }

        if self.wx >= DISPLAY_WIDTH as u8 + 7 || !self.is_window_display_enabled() {
            // If the window is used and a scan line interrupt
            // disables it (either by writing to LCDC or by setting
            // WX > 166) and a scan line interrupt a little later on
            // enables it then the window will resume appearing on
            // the screen at the exact position of the window where
            // it left off earlier. This way, even if there are only
            // 16 lines of useful graphics in the window, you could
            // display the first 8 lines at the top of the screen and
            // the next 8 lines at the bottom if you wanted to do so.
            self.wy_offset += 1;
            return;
        }

        let tile_data_section_start =
            (self.backround_window_tile_data_section_start() - MEM_AREA_VRAM_START) as usize;
        let tile_map_start =
            (self.window_tile_map_display_section_start() - MEM_AREA_VRAM_START) as usize;

        let actual_ly = ly as i16 - self.wy as i16 - self.wy_offset as i16;
        if actual_ly < 0 || actual_ly >= 0x100 {
            return;
        }

        let tile_row = actual_ly / 8;
        let tile_y = actual_ly % 8;

        for i in 0..DISPLAY_WIDTH {
            let actual_x = i as i16 - (self.wx as i16 - 7);
            if actual_x < 0 || actual_x >= 0x100 {
                continue;
            }

            let tile_col = actual_x / 8;
            let tile_x = (actual_x % 8) as u8;
            let tile_data_i = (tile_row as usize * 32) + tile_col as usize;
            let tile_i = self.vram[tile_map_start + tile_data_i];

            let tile_i = if tile_data_section_start == 0x0800 {
                tile_i.wrapping_add(128)
            } else {
                tile_i
            };

            let tile_lo =
                self.vram[tile_data_section_start + tile_i as usize * 16 + tile_y as usize * 2];
            let tile_hi =
                self.vram[tile_data_section_start + tile_i as usize * 16 + tile_y as usize * 2 + 1];
            let color = (bit(tile_hi, 7 - tile_x) << 1) | bit(tile_lo, 7 - tile_x);

            self.set_display_pixel(i as _, ly as _, self.bgp, color);
        }
    }

    fn is_vram_accessible(&self) -> bool {
        !self.is_lcd_display_enabled() || self.lcd_ppu_mode() != LcdPpuMode::M3
    }

    fn is_oam_accessible(&self) -> bool {
        !self.is_lcd_display_enabled()
            || (self.lcd_ppu_mode() != LcdPpuMode::M2 && self.lcd_ppu_mode() != LcdPpuMode::M3)
    }

    pub fn read(&self, loc: u16) -> Result<u8, Error> {
        let byte = match loc {
            MEM_AREA_VRAM_START..=MEM_AREA_VRAM_END => {
                if self.is_vram_accessible() {
                    self.vram[(loc - MEM_AREA_VRAM_START) as usize]
                } else {
                    0xFF
                }
            }
            MEM_AREA_OAM_START..=MEM_AREA_OAM_END => {
                if self.is_oam_accessible() {
                    self.oam_ram[(loc - MEM_AREA_OAM_START) as usize]
                } else {
                    0xFF
                }
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

        Ok(byte)
    }

    pub fn write(&mut self, loc: u16, byte: u8) {
        match loc {
            MEM_AREA_VRAM_START..=MEM_AREA_VRAM_END => {
                if self.is_vram_accessible() {
                    self.vram[(loc - MEM_AREA_VRAM_START) as usize] = byte;
                }
            }
            MEM_AREA_OAM_START..=MEM_AREA_OAM_END => {
                if self.is_oam_accessible() {
                    self.oam_ram[(loc - MEM_AREA_OAM_START) as usize] = byte;
                }
            }
            MEM_LOC_LCDC => {
                let lcdc_on_prev = self.is_lcd_display_enabled();
                self.lcdc = byte;
                let lcdc_on_curr = self.is_lcd_display_enabled();

                if lcdc_on_prev && !lcdc_on_curr {
                    self.stat_counter = 0;
                    self.ly = 0;
                    let _ = self.set_lcd_stat_ppu_mode(0);
                }
                if !lcdc_on_prev && lcdc_on_curr {
                    let _ = self.set_lcd_stat_ppu_mode(2);
                    self.stat_counter = 4;
                }
            }
            MEM_LOC_STAT => {
                self.stat = (byte | 0x80) & !0b11u8 /* Ignore mode bits */;
            }
            MEM_LOC_SCY => self.scy = byte,
            MEM_LOC_SCX => self.scx = byte,
            MEM_LOC_LY => panic!("Cannot write LY"),
            MEM_LOC_LYC => {
                self.lyc = byte;

                if self.lyc == self.ly {
                    self.lyc_change_interrupt = true;
                }
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
            self.oam_ram[i] = *byte;
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

    #[must_use]
    fn set_lcd_stat_ppu_mode(&mut self, mode: u8) -> bool {
        assert!(mode <= 0b11);
        self.stat &= 0b1111_1100;
        self.stat |= mode;

        match self.lcd_ppu_mode() {
            LcdPpuMode::M0 => self.is_mode0_hblank_interrupt_enabled(),
            LcdPpuMode::M1 => {
                self.wy_offset = 0;
                self.is_mode1_vblank_interrupt_enabled()
            }
            LcdPpuMode::M2 => self.is_mode2_oam_interrupt_enabled(),
            _ => false,
        }
    }

    fn ensure_fps(&mut self) {
        if self.ignore_fps_limiter {
            return;
        }

        let elapsed = self.fps_ctrl_time.elapsed().as_micros();

        // For performance debugging.
        // println!("{}", elapsed);

        if elapsed < ONE_FRAME_IN_MICROSECONDS as u128 {
            thread::sleep(Duration::from_micros(
                ONE_FRAME_IN_MICROSECONDS as u64 - elapsed as u64,
            ));
        }

        self.fps_ctrl_time = Instant::now();
    }

    pub fn fill_frame_buffer(&self, window_id: WindowId, frame: &mut [u8]) {
        if Some(window_id) == self.main_window_id {
            self.transfer_display_to_screen_buffer(frame);
        } else if Some(window_id) == self.tile_debug_window_id {
            self.draw_debug_tiles(frame);
        } else if Some(window_id) == self.background_debug_window_id {
            self.draw_debug_background(frame);
        } else if Some(window_id) == self.window_debug_window_id {
            self.draw_debug_window(frame);
        }
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
                        let gb_pixel_color = apply_palette(
                            (((byte2 >> (7 - sprite_x)) & 0b1) << 1)
                                | ((byte1 >> (7 - sprite_x)) & 0b1),
                            self.bgp,
                        );

                        let pixel_color = pixel_rgb8888_color(gb_pixel_color);

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

    pub fn draw_debug_background(&self, frame: &mut [u8]) {
        self.draw_debug_window_or_background(
            frame,
            (self.background_tile_map_display_section_start() - MEM_AREA_VRAM_START) as usize,
        );
    }

    pub fn draw_debug_window(&self, frame: &mut [u8]) {
        self.draw_debug_window_or_background(
            frame,
            (self.window_tile_map_display_section_start() - MEM_AREA_VRAM_START) as usize,
        );
    }

    pub fn draw_debug_window_or_background(&self, frame: &mut [u8], tile_map_start: usize) {
        let tile_data_section_start =
            (self.backround_window_tile_data_section_start() - MEM_AREA_VRAM_START) as usize;

        for y in 0..32usize {
            for x in 0..32usize {
                let tile_data_i = (y * 32) + x;
                let tile_i = if tile_data_section_start == 0x0800 {
                    self.vram[tile_map_start + tile_data_i].wrapping_add(128)
                } else {
                    self.vram[tile_map_start + tile_data_i]
                };

                for tile_y in 0..8u8 {
                    //                          32 tiles up      tile lines up              left     frame pixels
                    let tile_line_pos = (y * 32 * 8 * 8 + tile_y as usize * 32 * 8 + x * 8) * 4;

                    let tile_lo = self.vram
                        [tile_data_section_start + tile_i as usize * 16 + tile_y as usize * 2];
                    let tile_hi = self.vram
                        [tile_data_section_start + tile_i as usize * 16 + tile_y as usize * 2 + 1];

                    for tile_x in 0..8u8 {
                        let tile_pixel_addr = tile_line_pos + tile_x as usize * 4;
                        let color = apply_palette(
                            (bit(tile_hi, 7 - tile_x) << 1) | bit(tile_lo, 7 - tile_x),
                            self.bgp,
                        );
                        let pixel_color = pixel_rgb8888_color(color);

                        frame[tile_pixel_addr + 0] = pixel_color[0];
                        frame[tile_pixel_addr + 1] = pixel_color[1];
                        frame[tile_pixel_addr + 2] = pixel_color[2];
                        frame[tile_pixel_addr + 3] = pixel_color[3];
                    }
                }
            }
        }
    }

    pub fn transfer_display_to_screen_buffer(&self, frame: &mut [u8]) {
        if self.is_lcd_display_enabled() {
            frame.copy_from_slice(&self.display_buffer);
        } else {
            // TODO: Maybe use that "whiter than white" DBG color - instead of black.
            frame.iter_mut().for_each(|b| *b = 0);
        }
    }

    fn set_display_pixel(&mut self, x: usize, y: usize, palette: u8, raw_color: u8) {
        let rgb8888 = pixel_rgb8888_color(apply_palette(raw_color, palette));
        let offs = (y * DISPLAY_WIDTH as usize + x) << 2;
        self.display_buffer[offs] = rgb8888[0];
        self.display_buffer[offs + 1] = rgb8888[1];
        self.display_buffer[offs + 2] = rgb8888[2];
        self.display_buffer[offs + 3] = rgb8888[3];
    }

    fn is_display_pixel_color_zero(&self, x: usize, y: usize) -> bool {
        let offs = (y * DISPLAY_WIDTH as usize + x) << 2;

        // Practically we can just check the first byte as the colors don't share component values.
        self.display_buffer[offs] == PALETTE[0][0]
    }

    pub fn debug_oam(&self) {
        for i in 0..40usize {
            let addr = i * 4;

            print!(
                "\x1B[37m#{:04X}\x1B[0m \x1B[94m{:02X}\x1B[0m\x1B[94m{:02X}\x1B[0m\x1B[93m{:02X}\x1B[0m\x1B[96m{:02X}\x1B[0m  ",
                addr, self.oam_ram[addr + 0], self.oam_ram[addr + 1], self.oam_ram[addr + 2], self.oam_ram[addr + 3]
            );

            if i % 8 == 7 {
                println!("");
            }
        }

        println!("");
    }
}
