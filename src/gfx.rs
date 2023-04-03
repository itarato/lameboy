use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::conf::*;

use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

pub struct Gfx {
    vram: Vram,
    oam_ram: OamVram,
    pixel_size: usize,
    global_exit_flag: Arc<AtomicBool>,
}

impl Gfx {
    pub fn new(vram: Vram, oam_ram: OamVram, global_exit_flag: Arc<AtomicBool>) -> Self {
        Gfx {
            vram,
            oam_ram,
            pixel_size: 4,
            global_exit_flag,
        }
    }

    fn display_width(&self) -> usize {
        DISPLAY_WIDTH * self.pixel_size
    }

    fn display_height(&self) -> usize {
        DISPLAY_HEIGHT * self.pixel_size
    }

    pub fn run(&self) {
        let event_loop = EventLoop::new();
        let mut input = WinitInputHelper::new();
        let window = {
            let size = LogicalSize::new(self.display_width() as f64, self.display_height() as f64);
            WindowBuilder::new()
                .with_title("Lameboy")
                .with_inner_size(size)
                .with_min_inner_size(size)
                .build(&event_loop)
                .unwrap()
        };

        let pixels = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, &window);
            Pixels::new(
                self.display_width() as u32,
                self.display_height() as u32,
                surface_texture,
            )
            .expect("Failed instantiating Pixels")
        };

        let global_exit_flag = self.global_exit_flag.clone();

        event_loop.run(move |event, _, control_flow| {
            // Draw the current frame
            if let Event::RedrawRequested(_) = event {
                if let Err(err) = pixels.render() {
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

                // Update internal state and request a redraw
                window.request_redraw();
            }

            if global_exit_flag.load(Ordering::Acquire) {
                *control_flow = ControlFlow::Exit;
                return;
            }
        });
    }
}
