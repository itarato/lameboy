/**
 * Tiles: 8x8
 * Color id (pixel info): 0-3
 * For object, color-0 = transparent
 * Layers: background / window / object
 * Background: pointers to tiles
 * Object: 1 (8x8) or 2 (8x16) tiles
 * VRAM: (0x8000-0x97FF) 16 bytes sections x 384
 */
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::conf::*;

use log::error;
use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use winit_input_helper::WinitInputHelper;

struct Drawer {
    vram: Vram,
    oam_ram: OamVram,
    wram: Wram,
    canvas: CanvasT,
}

impl Drawer {
    fn new(vram: Vram, oam_ram: OamVram, wram: Wram, canvas: CanvasT) -> Drawer {
        Drawer {
            vram,
            oam_ram,
            wram,
            canvas,
        }
    }

    fn draw_debug_tiles(&self, frame: &mut [u8]) {
        let vram = self.vram.lock().expect("Cannot lock vram for debug print");
        const FRAME_LINE_OFFS: usize = 16 * 8 * 4;

        for y in 0..24 {
            for x in 0..16 {
                let tile_number = (y * 16) + x;
                let vram_pos = tile_number * 16; // 8x8 pixel with 2bpp = 16 bytes
                let frame_pos = (y * 8 * 8 * 4 * 16) + (x * 8 * 4); // Assuming frame is 4-attr color (RGBA) * 8x8 sprite size
                for sprite_y in 0..8 {
                    let byte1 = vram[vram_pos + sprite_y * 2];
                    let byte2 = vram[vram_pos + sprite_y * 2 + 1];
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

    fn draw_display(&self, frame: &mut [u8]) {
        let canvas = self.canvas.lock().expect("Cannot lock canvas for drawing");

        for y in 0..DISPLAY_HEIGHT {
            for x in 0..DISPLAY_WIDTH {
                let pixel_pos: usize = ((y * DISPLAY_WIDTH) + x) as usize;
                let frame_pos: usize = pixel_pos * 4;
                let pixel_color = self.pixel_color(canvas[pixel_pos]);

                frame[frame_pos + 0] = pixel_color[0];
                frame[frame_pos + 1] = pixel_color[1];
                frame[frame_pos + 2] = pixel_color[2];
                frame[frame_pos + 3] = pixel_color[3];
            }
        }
    }

    fn pixel_color(&self, code: u8) -> [u8; 4] {
        match code {
            0b00 => [0x10, 0x40, 0x20, 0xff],
            0b01 => [0x10, 0x80, 0x40, 0xff],
            0b10 => [0x10, 0xa0, 0x50, 0xff],
            0b11 => [0x10, 0xf0, 0x80, 0xff],
            _ => unimplemented!("Unknown gb pixel color"),
        }
    }
}

pub struct Gfx {
    global_exit_flag: Arc<AtomicBool>,
}

impl Gfx {
    pub fn new(global_exit_flag: Arc<AtomicBool>) -> Self {
        Gfx { global_exit_flag }
    }

    fn make_main_window(&self, event_loop: &EventLoop<()>) -> (Window, Pixels) {
        let size = LogicalSize::new(DISPLAY_WIDTH as f64, DISPLAY_HEIGHT as f64);
        let window = WindowBuilder::new()
            .with_title("Lameboy")
            .with_inner_size(size.to_physical::<f64>(4.0))
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap();

        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let pixels = Pixels::new(DISPLAY_WIDTH, DISPLAY_HEIGHT, surface_texture)
            .expect("Failed instantiating Pixels");

        (window, pixels)
    }

    // 8x8 tiles
    // 16 tiles wide
    // 24 tiles tall
    fn make_tile_debug_window(&self, event_loop: &EventLoop<()>) -> (Window, Pixels) {
        let size = LogicalSize::new((8 * 16) as f64, (8 * 24) as f64);
        let window = WindowBuilder::new()
            .with_title("Tile debug")
            .with_inner_size(size.to_physical::<f64>(4.0))
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap();

        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let pixels =
            Pixels::new(8 * 16, 8 * 24, surface_texture).expect("Failed instantiating Pixels");

        (window, pixels)
    }

    pub fn run(&self, vram: Vram, oam_ram: OamVram, wram: Wram, canvas: CanvasT) {
        let drawer = Drawer::new(vram, oam_ram, wram, canvas);

        let event_loop = EventLoop::new();
        let mut input = WinitInputHelper::new();
        let (window, mut pixels) = self.make_main_window(&event_loop);
        let (tile_debug_window, mut pixels_for_tile_debug_window) =
            self.make_tile_debug_window(&event_loop);

        let global_exit_flag = self.global_exit_flag.clone();

        event_loop.run(move |event, _, control_flow| {
            // Draw the current frame
            if let Event::RedrawRequested(_) = event {
                drawer.draw_display(pixels.frame_mut());
                if let Err(err) = pixels.render() {
                    global_exit_flag.store(false, Ordering::Release);
                    *control_flow = ControlFlow::Exit;
                    return;
                }

                drawer.draw_debug_tiles(pixels_for_tile_debug_window.frame_mut());
                if let Err(err) = pixels_for_tile_debug_window.render() {
                    global_exit_flag.store(false, Ordering::Release);
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }

            // Handle input events
            if input.update(&event) {
                // Close events
                if input.key_pressed(VirtualKeyCode::Escape)
                    || input.close_requested()
                    || input.destroyed()
                {
                    global_exit_flag.store(false, Ordering::Release);
                    *control_flow = ControlFlow::Exit;
                    return;
                }

                // Resize the window
                if let Some(size) = input.window_resized() {
                    if let Err(err) = pixels.resize_surface(size.width, size.height) {
                        error!("pixels.resize_surface error: {}", err);
                        *control_flow = ControlFlow::Exit;
                        return;
                    }

                    if let Err(err) =
                        pixels_for_tile_debug_window.resize_surface(size.width, size.height)
                    {
                        error!("pixels.resize_surface error: {}", err);
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                }

                // Update internal state and request a redraw
                window.request_redraw();
                tile_debug_window.request_redraw();
            }

            if global_exit_flag.load(Ordering::Acquire) {
                *control_flow = ControlFlow::Exit;
                return;
            }
        });
    }
}
