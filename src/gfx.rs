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

pub struct Gfx {
    vram: Vram,
    oam_ram: OamVram,
    wram: Wram,
    global_exit_flag: Arc<AtomicBool>,
}

impl Gfx {
    pub fn new(
        vram: Vram,
        oam_ram: OamVram,
        wram: Wram,
        global_exit_flag: Arc<AtomicBool>,
    ) -> Self {
        Gfx {
            vram,
            oam_ram,
            wram,
            global_exit_flag,
        }
    }

    fn display_width(&self) -> u32 {
        DISPLAY_WIDTH
    }

    fn display_height(&self) -> u32 {
        DISPLAY_HEIGHT
    }

    fn make_main_window(&self, event_loop: &EventLoop<()>) -> (Window, Pixels) {
        let size = LogicalSize::new(self.display_width() as f64, self.display_height() as f64);
        let window = WindowBuilder::new()
            .with_title("Lameboy")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap();

        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let pixels = Pixels::new(self.display_width(), self.display_height(), surface_texture)
            .expect("Failed instantiating Pixels");

        (window, pixels)
    }

    fn make_tile_debug_window(&self, event_loop: &EventLoop<()>) -> (Window, Pixels) {
        let size = LogicalSize::new(self.display_width() as f64, self.display_height() as f64);
        let window = WindowBuilder::new()
            .with_title("Tile debug")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap();

        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let pixels = Pixels::new(self.display_width(), self.display_height(), surface_texture)
            .expect("Failed instantiating Pixels");

        (window, pixels)
    }

    pub fn run(&self) {
        let event_loop = EventLoop::new();
        let mut input = WinitInputHelper::new();
        let (window, mut pixels) = self.make_main_window(&event_loop);
        let (tile_debug_window, mut pixels_for_tile_debug_window) =
            self.make_tile_debug_window(&event_loop);

        let global_exit_flag = self.global_exit_flag.clone();

        event_loop.run(move |event, _, control_flow| {
            // Draw the current frame
            if let Event::RedrawRequested(_) = event {
                if let Err(err) = pixels.render() {
                    global_exit_flag.store(false, Ordering::Release);
                    *control_flow = ControlFlow::Exit;
                    return;
                }

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
