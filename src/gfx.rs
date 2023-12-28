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
    Arc, RwLock,
};

use crate::{conf::*, video::Video};

use log::error;
use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};
use winit_input_helper::WinitInputHelper;

pub struct Gfx {
    global_exit_flag: Arc<AtomicBool>,
    video: Arc<RwLock<Video>>,
}

impl Gfx {
    pub fn new(global_exit_flag: Arc<AtomicBool>, video: Arc<RwLock<Video>>) -> Self {
        Gfx {
            global_exit_flag,
            video,
        }
    }

    fn make_main_window(&self, event_loop: &EventLoop<()>) -> (Window, Pixels) {
        let size = LogicalSize::new(DISPLAY_WIDTH as f64, DISPLAY_HEIGHT as f64);
        let window = WindowBuilder::new()
            .with_title("Lameboy")
            .with_inner_size(size.to_physical::<f64>(2.0))
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
            .with_inner_size(size.to_physical::<f64>(2.0))
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap();

        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let pixels =
            Pixels::new(8 * 16, 8 * 24, surface_texture).expect("Failed instantiating Pixels");

        (window, pixels)
    }

    pub fn run(&self) {
        let event_loop = EventLoop::new();
        let mut input = WinitInputHelper::new();
        let (tile_debug_window, mut pixels_for_tile_debug_window) =
            self.make_tile_debug_window(&event_loop);
        let (main_window, mut pixels) = self.make_main_window(&event_loop);

        let global_exit_flag = self.global_exit_flag.clone();
        let video = self.video.clone();

        event_loop.run(move |event, _, control_flow| {
            match &event {
                Event::WindowEvent { window_id, event } => match event {
                    WindowEvent::Resized(size) => {
                        if window_id == &main_window.id() {
                            if let Err(err) = pixels.resize_surface(size.width, size.height) {
                                error!("pixels.resize_surface error: {}", err);
                                control_flow.set_exit();
                                return;
                            }
                        }

                        if window_id == &tile_debug_window.id() {
                            if let Err(err) =
                                pixels_for_tile_debug_window.resize_surface(size.width, size.height)
                            {
                                error!("pixels.resize_surface error: {}", err);
                                control_flow.set_exit();
                                return;
                            }
                        }
                    }
                    _ => {}
                },
                Event::RedrawRequested(window_id) => {
                    if window_id == &main_window.id() {
                        video.read().unwrap().draw_display(pixels.frame_mut());
                        if let Err(_) = pixels.render() {
                            global_exit_flag.store(false, Ordering::Release);
                            control_flow.set_exit();
                            return;
                        }
                    }

                    if window_id == &tile_debug_window.id() {
                        video
                            .read()
                            .unwrap()
                            .draw_debug_tiles(pixels_for_tile_debug_window.frame_mut());
                        if let Err(_) = pixels_for_tile_debug_window.render() {
                            global_exit_flag.store(false, Ordering::Release);
                            control_flow.set_exit();
                            return;
                        }
                    }
                }
                _ => {}
            };

            // Handle input events
            if input.update(&event) {
                // Close events
                if input.key_pressed(VirtualKeyCode::Escape)
                    || input.close_requested()
                    || input.destroyed()
                {
                    global_exit_flag.store(false, Ordering::Release);
                    control_flow.set_exit();
                    return;
                }

                // Update internal state and request a redraw
                main_window.request_redraw();
                tile_debug_window.request_redraw();
            }

            if global_exit_flag.load(Ordering::Acquire) {
                control_flow.set_exit();
                return;
            }
        });
    }
}
