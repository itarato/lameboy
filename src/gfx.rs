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
    time::Instant,
};

use crate::{conf::*, joypad::JoypadInputRequest, ppu::PPU};

use log::error;
use pixels::{
    wgpu::{
        CommandEncoder, LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor,
        TextureView,
    },
    Pixels, PixelsContext, SurfaceTexture,
};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};
use winit_input_helper::WinitInputHelper;

struct ImguiService {
    imgui: imgui::Context,
    platform: imgui_winit_support::WinitPlatform,
    renderer: imgui_wgpu::Renderer,
    last_frame: Instant,
    last_cursor: Option<imgui::MouseCursor>,
    show_ui: bool,
    vm_debug_log: Arc<RwLock<Vec<String>>>,
    global_should_generate_vm_debug_log: Arc<AtomicBool>,
}

impl ImguiService {
    fn new(
        window: &Window,
        pixels: &Pixels,
        show_ui: bool,
        vm_debug_log: Arc<RwLock<Vec<String>>>,
        global_should_generate_vm_debug_log: Arc<AtomicBool>,
    ) -> ImguiService {
        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);

        // There is a bug in wgpu crate that messes up rendering. Something with HiDPI settings. For now it's locked.
        // @link https://github.com/Yatekii/imgui-wgpu-rs/issues/77
        let scale = window.scale_factor().round();

        let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
        platform.attach_window(
            imgui.io_mut(),
            window,
            // See bug below.
            imgui_winit_support::HiDpiMode::Locked(scale),
        );

        let hidpi_factor = scale;
        let font_size = (13.0 * hidpi_factor) as f32;

        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;
        imgui
            .fonts()
            .add_font(&[imgui::FontSource::DefaultFontData {
                config: Some(imgui::FontConfig {
                    oversample_h: 1,
                    pixel_snap_h: true,
                    size_pixels: font_size,
                    ..Default::default()
                }),
            }]);

        let device = pixels.device();
        let queue = pixels.queue();
        let config = imgui_wgpu::RendererConfig {
            texture_format: pixels.render_texture_format(),
            ..Default::default()
        };
        let renderer = imgui_wgpu::Renderer::new(&mut imgui, device, queue, config);

        ImguiService {
            imgui,
            platform,
            renderer,
            last_frame: Instant::now(),
            last_cursor: None,
            show_ui,
            vm_debug_log,
            global_should_generate_vm_debug_log,
        }
    }

    pub fn prepare(&mut self, window: &Window) -> Result<(), winit::error::ExternalError> {
        let now = Instant::now();
        self.imgui.io_mut().update_delta_time(now - self.last_frame);
        self.last_frame = now;
        self.platform.prepare_frame(self.imgui.io_mut(), window)
    }

    pub fn render(
        &mut self,
        window: &Window,
        encoder: &mut CommandEncoder,
        target: &TextureView,
        context: &PixelsContext,
    ) -> imgui_wgpu::RendererResult<()> {
        let ui = self.imgui.new_frame();

        let mouse_cursor = ui.mouse_cursor();
        if self.last_cursor != mouse_cursor {
            self.last_cursor = mouse_cursor;
            self.platform.prepare_render(ui, window);
        }

        if self.show_ui {
            ui.window("VM Debug")
                .position([0.0, 0.0], imgui::Condition::Once)
                .size([220.0, 240.0], imgui::Condition::FirstUseEver)
                .opened(&mut self.show_ui)
                .build(|| {
                    self.vm_debug_log
                        .read()
                        .unwrap()
                        .iter()
                        .for_each(|line| ui.text(line));
                });

            if !self.show_ui {
                self.global_should_generate_vm_debug_log
                    .store(false, Ordering::Relaxed);
            }
        }

        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("imgui"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        self.renderer.render(
            self.imgui.render(),
            &context.queue,
            &context.device,
            &mut render_pass,
        )
    }

    pub fn handle_event(&mut self, window: &Window, event: &Event<()>) {
        self.platform
            .handle_event(self.imgui.io_mut(), window, event);
    }
}

fn make_window(
    event_loop: &EventLoop<()>,
    title: &str,
    width: u32,
    height: u32,
    visible: bool,
) -> (Window, Pixels) {
    let size = LogicalSize::new(width as f64, height as f64);
    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(size.to_physical::<f64>(2.0))
        .with_min_inner_size(size)
        .with_visible(visible)
        .build(&event_loop)
        .unwrap();

    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    let pixels = Pixels::new(width, height, surface_texture).expect("Failed instantiating Pixels");

    (window, pixels)
}

pub fn run(
    global_exit_flag: Arc<AtomicBool>,
    video: Arc<RwLock<PPU>>,
    breakpoint_flag: Arc<AtomicBool>,
    buttons: Arc<RwLock<JoypadInputRequest>>,
    with_tile_debug_window: bool,
    with_background_debug_window: bool,
    with_window_debug_window: bool,
    vm_debug_log: Arc<RwLock<Vec<String>>>,
    global_should_generate_vm_debug_log: Arc<AtomicBool>,
    catridge_title: String,
) {
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let mut show_tiles = with_tile_debug_window;
    let mut show_bg = with_background_debug_window;
    let mut show_win = with_window_debug_window;

    let mut pixels_map = HashMap::new();

    let (tile_window, tile_pixels) =
        make_window(&event_loop, "(1) VRAM Tile Map", 8 * 16, 8 * 24, show_tiles);
    let (bg_window, bg_pixels) = make_window(
        &event_loop,
        "(2) Background map (32 x 32)",
        256,
        256,
        show_bg,
    );
    let (win_window, win_pixels) =
        make_window(&event_loop, "(3) Window map (32 x 32)", 256, 256, show_win);
    let (main_window, main_pixels) = make_window(
        &event_loop,
        format!("Lameboy <{}>", catridge_title).as_str(),
        DISPLAY_WIDTH,
        DISPLAY_HEIGHT,
        true,
    );

    video.write().unwrap().main_window_id = Some(main_window.id());
    video.write().unwrap().tile_debug_window_id = Some(tile_window.id());
    video.write().unwrap().background_debug_window_id = Some(bg_window.id());
    video.write().unwrap().window_debug_window_id = Some(win_window.id());

    let mut imgui_service = ImguiService::new(
        &main_window,
        &main_pixels,
        false,
        vm_debug_log,
        global_should_generate_vm_debug_log.clone(),
    );

    pixels_map.insert(tile_window.id(), tile_pixels);
    pixels_map.insert(bg_window.id(), bg_pixels);
    pixels_map.insert(win_window.id(), win_pixels);
    pixels_map.insert(main_window.id(), main_pixels);

    let main_window_id = main_window.id();

    let global_exit_flag = global_exit_flag.clone();

    event_loop.run(move |event, _, control_flow| {
        match &event {
            Event::WindowEvent { window_id, event } => match event {
                WindowEvent::Resized(size) => {
                    if let Some(pixels) = pixels_map.get_mut(window_id) {
                        if let Err(err) = pixels.resize_surface(size.width, size.height) {
                            error!("pixels.resize_surface error: {}", err);
                            // This is likely a bug in OS-X + wgpu/pixels: pixels.resize_surface error: Texture width is invalid: 4294967295
                            // However the program can run almost fine with this error. Allowing it for now.
                            // control_flow.set_exit();
                            // return;
                        }
                    }
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) => {
                if let Some(pixels) = pixels_map.get_mut(window_id) {
                    video
                        .read()
                        .unwrap()
                        .fill_frame_buffer(*window_id, pixels.frame_mut());

                    if *window_id == main_window_id {
                        imgui_service
                            .prepare(&main_window)
                            .expect("Failed preparing ImGUI context");

                        let _render_result = pixels.render_with(|encoder, target, context| {
                            context.scaling_renderer.render(encoder, target);
                            imgui_service
                                .render(&main_window, encoder, target, context)
                                .expect("Failed rendering ImGUI");
                            Ok(())
                        });
                    } else {
                        if let Err(_) = pixels.render() {
                            global_exit_flag.store(false, Ordering::Release);
                            control_flow.set_exit();
                            return;
                        }
                    }
                }
            }
            _ => {}
        };

        // Handle input events
        imgui_service.handle_event(&main_window, &event);
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                global_exit_flag.store(false, Ordering::Release);
                control_flow.set_exit();
                return;
            }

            if input.key_released(VirtualKeyCode::I) {
                imgui_service.show_ui = !imgui_service.show_ui;
                global_should_generate_vm_debug_log.store(imgui_service.show_ui, Ordering::Relaxed);
            }

            if input.key_released(VirtualKeyCode::Key1) {
                show_tiles = !show_tiles;
                tile_window.set_visible(show_tiles);
            }
            if input.key_released(VirtualKeyCode::Key2) {
                show_bg = !show_bg;
                bg_window.set_visible(show_bg);
            }
            if input.key_released(VirtualKeyCode::Key3) {
                show_win = !show_win;
                win_window.set_visible(show_win);
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

            if input.key_pressed(VirtualKeyCode::Up) {
                buttons.write().expect("Cannot lock buttons").up = true;
            }
            if input.key_pressed(VirtualKeyCode::Down) {
                buttons.write().expect("Cannot lock buttons").down = true;
            }
            if input.key_pressed(VirtualKeyCode::Left) {
                buttons.write().expect("Cannot lock buttons").left = true;
            }
            if input.key_pressed(VirtualKeyCode::Right) {
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

            if input.key_released(VirtualKeyCode::Up) {
                buttons.write().expect("Cannot lock buttons").up = false;
            }
            if input.key_released(VirtualKeyCode::Down) {
                buttons.write().expect("Cannot lock buttons").down = false;
            }
            if input.key_released(VirtualKeyCode::Left) {
                buttons.write().expect("Cannot lock buttons").left = false;
            }
            if input.key_released(VirtualKeyCode::Right) {
                buttons.write().expect("Cannot lock buttons").right = false;
            }

            let main_window_had_updates = match video
                .read()
                .unwrap()
                .display_finished
                .compare_exchange_weak(true, false, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => true,
                Err(_) => false,
            };
            if main_window_had_updates || imgui_service.show_ui {
                main_window.request_redraw();
            }

            if show_tiles {
                tile_window.request_redraw();
            }
            if show_bg {
                bg_window.request_redraw();
            }
            if show_win {
                win_window.request_redraw();
            }
        }

        if global_exit_flag.load(Ordering::Acquire) {
            control_flow.set_exit();
            return;
        }
    });
}
