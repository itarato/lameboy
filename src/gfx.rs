/**
 * Tiles: 8x8
 * Color id (pixel info): 0-3
 * For object, color-0 = transparent
 * Layers: background / window / object
 * Background: pointers to tiles
 * Object: 1 (8x8) or 2 (8x16) tiles
 * VRAM: (0x8000-0x97FF) 16 bytes sections x 384
 */
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
};

use crate::{conf::*, joypad::JoypadInputRequest, ppu::PPU};

use log::error;
use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};
use winit_input_helper::WinitInputHelper;

fn make_main_window(event_loop: &EventLoop<()>) -> (Window, Pixels) {
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
fn make_tile_debug_window(event_loop: &EventLoop<()>) -> (Window, Pixels) {
    let size = LogicalSize::new((8 * 16) as f64, (8 * 24) as f64);
    let window = WindowBuilder::new()
        .with_title("Tile debug")
        .with_inner_size(size.to_physical::<f64>(2.0))
        .with_min_inner_size(size)
        .build(&event_loop)
        .unwrap();

    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    let pixels = Pixels::new(8 * 16, 8 * 24, surface_texture).expect("Failed instantiating Pixels");

    (window, pixels)
}

pub fn run(
    global_exit_flag: Arc<AtomicBool>,
    video: Arc<RwLock<PPU>>,
    breakpoint_flag: Arc<AtomicBool>,
    buttons: Arc<RwLock<JoypadInputRequest>>,
    with_vram_debug_window: bool,
) {
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let mut windows = HashMap::new();

    if with_vram_debug_window {
        let (tile_debug_window, pixels_for_tile_debug_window) = make_tile_debug_window(&event_loop);
        video.write().unwrap().vram_debug_window_id = Some(tile_debug_window.id());
        windows.insert(
            tile_debug_window.id(),
            (tile_debug_window, pixels_for_tile_debug_window),
        );
    }

    let (main_window, pixels) = make_main_window(&event_loop);
    video.write().unwrap().main_window_id = Some(main_window.id());
    windows.insert(main_window.id(), (main_window, pixels));

    let global_exit_flag = global_exit_flag.clone();

    event_loop.run(move |event, _, control_flow| {
        match &event {
            Event::WindowEvent { window_id, event } => match event {
                WindowEvent::Resized(size) => {
                    if let Some((_window, pixels)) = windows.get_mut(window_id) {
                        if let Err(err) = pixels.resize_surface(size.width, size.height) {
                            error!("pixels.resize_surface error: {}", err);
                            control_flow.set_exit();
                            return;
                        }
                    }
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) => {
                if let Some((_window, pixels)) = windows.get_mut(window_id) {
                    video
                        .read()
                        .unwrap()
                        .fill_frame_buffer(*window_id, pixels.frame_mut());
                    if let Err(_) = pixels.render() {
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

            if input.key_pressed(VirtualKeyCode::B) {
                breakpoint_flag.store(true, Ordering::Relaxed);
            }

            if input.key_pressed(VirtualKeyCode::Z) {
                buttons.write().expect("Cannot lock buttons").start = true;
            }
            if input.key_pressed(VirtualKeyCode::X) {
                buttons.write().expect("Cannot lock buttons").select = true;
            }
            if input.key_pressed(VirtualKeyCode::N) {
                buttons.write().expect("Cannot lock buttons").a = true;
            }
            if input.key_pressed(VirtualKeyCode::M) {
                buttons.write().expect("Cannot lock buttons").b = true;
            }

            if input.key_pressed(VirtualKeyCode::W) {
                buttons.write().expect("Cannot lock buttons").up = true;
            }
            if input.key_pressed(VirtualKeyCode::S) {
                buttons.write().expect("Cannot lock buttons").down = true;
            }
            if input.key_pressed(VirtualKeyCode::A) {
                buttons.write().expect("Cannot lock buttons").left = true;
            }
            if input.key_pressed(VirtualKeyCode::D) {
                buttons.write().expect("Cannot lock buttons").right = true;
            }

            if input.key_released(VirtualKeyCode::Z) {
                buttons.write().expect("Cannot lock buttons").start = false;
            }
            if input.key_released(VirtualKeyCode::X) {
                buttons.write().expect("Cannot lock buttons").select = false;
            }
            if input.key_released(VirtualKeyCode::N) {
                buttons.write().expect("Cannot lock buttons").a = false;
            }
            if input.key_released(VirtualKeyCode::M) {
                buttons.write().expect("Cannot lock buttons").b = false;
            }

            if input.key_released(VirtualKeyCode::W) {
                buttons.write().expect("Cannot lock buttons").up = false;
            }
            if input.key_released(VirtualKeyCode::S) {
                buttons.write().expect("Cannot lock buttons").down = false;
            }
            if input.key_released(VirtualKeyCode::A) {
                buttons.write().expect("Cannot lock buttons").left = false;
            }
            if input.key_released(VirtualKeyCode::D) {
                buttons.write().expect("Cannot lock buttons").right = false;
            }

            // Update internal state and request a redraw
            for (_id, (window, _pixels)) in &windows {
                window.request_redraw();
            }
        }

        if global_exit_flag.load(Ordering::Acquire) {
            control_flow.set_exit();
            return;
        }
    });
}
