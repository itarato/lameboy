/**
 * Tiles: 8x8
 * Color id (pixel info): 0-3
 * For object, color-0 = transparent
 * Layers: background / window / object
 * Background: pointers to tiles
 * Object: 1 (8x8) or 2 (8x16) tiles
 * VRAM: (0x8000-0x97FF) 16 bytes sections x 384
 */
use std::{cell::RefCell, rc::Rc};

use crate::{joypad::JoypadInputRequest, ppu::PPU};

use sdl2::{
    event::Event,
    keyboard::Keycode,
    render::{Canvas, Texture},
    video::Window,
    EventPump, Sdl, VideoSubsystem,
};

pub struct PixelWindowBundle<'t> {
    canvas: Canvas<Window>,
    texture: Texture<'t>,
}

impl<'t> PixelWindowBundle<'t> {
    pub fn new(canvas: Canvas<Window>, texture: Texture<'t>) -> PixelWindowBundle {
        PixelWindowBundle { canvas, texture }
    }
}

pub struct Gfx<'a> {
    event_pump: EventPump,
    breakpoint_requested: Rc<RefCell<bool>>,
    quit_requested: Rc<RefCell<bool>>,
    joypad_input: Rc<RefCell<JoypadInputRequest>>,
    main_window: PixelWindowBundle<'a>,
    tile_window: Option<PixelWindowBundle<'a>>,
    background_window: Option<PixelWindowBundle<'a>>,
    window_window: Option<PixelWindowBundle<'a>>,
}

impl<'a> Gfx<'a> {
    pub fn new(
        sdl2_context: &Sdl,
        sdl2_video: &VideoSubsystem,
        breakpoint_requested: Rc<RefCell<bool>>,
        quit_requested: Rc<RefCell<bool>>,
        joypad_input: Rc<RefCell<JoypadInputRequest>>,
        main_window: PixelWindowBundle<'a>,
        tile_window: Option<PixelWindowBundle<'a>>,
        background_window: Option<PixelWindowBundle<'a>>,
        window_window: Option<PixelWindowBundle<'a>>,
    ) -> Gfx<'a> {
        let event_pump = sdl2_context.event_pump().unwrap();

        Gfx {
            event_pump,
            breakpoint_requested,
            quit_requested,
            joypad_input,
            main_window,
            tile_window,
            background_window,
            window_window,
        }
    }

    pub fn frame(&mut self, video: &mut PPU) {
        let mut out = 0;
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => *self.quit_requested.borrow_mut() = true,
                Event::KeyDown { keycode, .. } => match keycode {
                    Some(Keycode::Escape) => *self.quit_requested.borrow_mut() = true,
                    Some(Keycode::B) => *self.breakpoint_requested.borrow_mut() = true,
                    Some(Keycode::Z) => (*self.joypad_input.borrow_mut()).start = true,
                    Some(Keycode::X) => (*self.joypad_input.borrow_mut()).select = true,
                    Some(Keycode::N) => (*self.joypad_input.borrow_mut()).a = true,
                    Some(Keycode::M) => (*self.joypad_input.borrow_mut()).b = true,
                    Some(Keycode::W) => (*self.joypad_input.borrow_mut()).up = true,
                    Some(Keycode::S) => (*self.joypad_input.borrow_mut()).down = true,
                    Some(Keycode::A) => (*self.joypad_input.borrow_mut()).left = true,
                    Some(Keycode::D) => (*self.joypad_input.borrow_mut()).right = true,
                    _ => {}
                },
                Event::KeyUp { keycode, .. } => match keycode {
                    Some(Keycode::Z) => (*self.joypad_input.borrow_mut()).start = false,
                    Some(Keycode::X) => (*self.joypad_input.borrow_mut()).select = false,
                    Some(Keycode::N) => (*self.joypad_input.borrow_mut()).a = false,
                    Some(Keycode::M) => (*self.joypad_input.borrow_mut()).b = false,
                    Some(Keycode::W) => (*self.joypad_input.borrow_mut()).up = false,
                    Some(Keycode::S) => (*self.joypad_input.borrow_mut()).down = false,
                    Some(Keycode::A) => (*self.joypad_input.borrow_mut()).left = false,
                    Some(Keycode::D) => (*self.joypad_input.borrow_mut()).right = false,
                    _ => {}
                },
                _ => {}
            };
        }

        video.draw_display(&mut self.main_window.texture);

        self.main_window
            .canvas
            .copy(&self.main_window.texture, None, None);
        self.main_window.canvas.present();
    }
}
