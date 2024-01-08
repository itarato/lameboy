mod apu;
mod cartridge;
mod conf;
mod cpu;
mod debugger;
mod gfx;
mod joypad;
mod mmu;
mod ppu;
mod serial;
mod timer;
mod util;
mod vm;

use std::cell::RefCell;
use std::rc::Rc;

use crate::cartridge::*;
use crate::conf::*;
use crate::debugger::*;
use crate::gfx::Gfx;
use crate::gfx::PixelWindowBundle;
use crate::ppu::PPU;
use crate::vm::*;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Cartridge.
    cartridge: String,

    /// Breakpoints.
    #[arg(short = 'b', long)]
    breakpoint: Option<String>,

    /// Step by step.
    #[arg(short = 's', long)]
    step_by_step: bool,

    /// Skip FPS limiter.
    #[arg(short, long)]
    nofps: bool,

    /// Dump opcode list to file.
    #[arg(long)]
    opcode_dump: bool,

    /// Tile map debug window.
    #[arg(long)]
    tiles: bool,

    /// Background map debug window.
    #[arg(long)]
    background: bool,

    /// Window map debug window.
    #[arg(long)]
    window: bool,

    /// Skip intro logo scrolling phase.
    #[arg(long)]
    skip_intro: bool,

    /// Turn all sounds off.
    #[arg(long)]
    disable_sound: bool,
}

impl Args {
    fn breakpoint_parsed(&self) -> Option<u16> {
        self.breakpoint.as_ref().map(|breakpoint| {
            u16::from_str_radix(breakpoint.as_str(), 16)
                .expect("Failed converting base-16 breakpoint address")
        })
    }
}

fn main() -> Result<(), Error> {
    simple_logger::SimpleLogger::new()
        .env()
        .with_module_level("wgpu_core", log::LevelFilter::Error)
        .init()
        .unwrap();
    log::info!("Emulation start");

    let args = Args::parse();

    let breakpoint_requested = Rc::new(RefCell::new(false));
    let quit_requested = Rc::new(RefCell::new(false));
    let mut debugger = Debugger::new(breakpoint_requested.clone());

    args.breakpoint_parsed()
        .map(|breakpoint| debugger.add_breakpoint(breakpoint));
    if args.step_by_step {
        debugger.set_break_on_start();
        debugger.set_step_by_step();
    }

    let joypad_button_input_requester = Rc::new(RefCell::new(joypad::JoypadInputRequest::new()));

    // GFX SETUP ///////////////////////////////////////////////////////////////
    let sdl2_context = sdl2::init().expect("Failed creating SDL2 context");
    let sdl2_video = sdl2_context.video().unwrap();

    // Main window.
    let (title, width, height) = ("Lameboy 0.0", 160, 144);
    let window = sdl2_video
        .window(title, width, height)
        .vulkan()
        .resizable()
        .build()
        .unwrap();
    let canvas = window.into_canvas().build().unwrap();
    let surface =
        sdl2::surface::Surface::new(width, height, sdl2::pixels::PixelFormatEnum::RGB888).unwrap();
    let texture_creator = canvas.texture_creator();
    let texture = texture_creator
        .create_texture_from_surface(surface)
        .unwrap();
    let main_window_bundle = PixelWindowBundle::new(canvas, texture);

    let tile_window_bundle = if args.tiles {
        // Tile debug window.
        let (title, width, height) = ("VRAM Tile Map", 8 * 16, 8 * 24);
        let window = sdl2_video
            .window(title, width, height)
            .vulkan()
            .resizable()
            .build()
            .unwrap();
        let canvas = window.into_canvas().build().unwrap();
        let surface =
            sdl2::surface::Surface::new(width, height, sdl2::pixels::PixelFormatEnum::RGB888)
                .unwrap();
        let texture_creator = canvas.texture_creator();
        let texture = texture_creator
            .create_texture_from_surface(surface)
            .unwrap();
        Some(PixelWindowBundle::new(canvas, texture))
    } else {
        None
    };

    // Tile debug window.
    let (title, width, height) = ("Background map (32 x 32)", 256, 256);
    let window = sdl2_video
        .window(title, width, height)
        .vulkan()
        .resizable()
        .build()
        .unwrap();
    let canvas = window.into_canvas().build().unwrap();
    let surface =
        sdl2::surface::Surface::new(width, height, sdl2::pixels::PixelFormatEnum::RGB888).unwrap();
    let texture_creator = canvas.texture_creator();
    let texture = texture_creator
        .create_texture_from_surface(surface)
        .unwrap();
    let background_window_bundle = Some(PixelWindowBundle::new(canvas, texture));

    // Tile debug window.
    let (title, width, height) = ("Window map (32 x 32)", 256, 256);
    let window = sdl2_video
        .window(title, width, height)
        .vulkan()
        .resizable()
        .build()
        .unwrap();
    let canvas = window.into_canvas().build().unwrap();
    let surface =
        sdl2::surface::Surface::new(width, height, sdl2::pixels::PixelFormatEnum::RGB888).unwrap();
    let texture_creator = canvas.texture_creator();
    let texture = texture_creator
        .create_texture_from_surface(surface)
        .unwrap();
    let window_window_bundle = Some(PixelWindowBundle::new(canvas, texture));

    let gfx = Gfx::new(
        &sdl2_context,
        &sdl2_video,
        breakpoint_requested.clone(),
        quit_requested.clone(),
        joypad_button_input_requester.clone(),
        main_window_bundle,
        tile_window_bundle,
        background_window_bundle,
        window_window_bundle,
    );
    // GFX SETUP END ///////////////////////////////////////////////////////////

    let video = PPU::new(args.nofps, gfx);
    let joypad = joypad::Joypad::new(joypad_button_input_requester.clone());

    if let Ok(mut vm) = VM::new(
        Cartridge::new(args.cartridge).expect("Cannot open cartridge"),
        debugger,
        video,
        args.opcode_dump,
        joypad,
        args.disable_sound,
        quit_requested.clone(),
    ) {
        vm.setup(args.skip_intro)?;

        if let Err(err) = vm.run() {
            vm.dump_op_history();
            return Err(err);
        }
    }

    // gfx::run(
    //     global_exit_flag.clone(),
    //     video.clone(),
    //     breakpoint_flag,
    //     joypad_button_input_requester,
    //     args.tiles,
    //     args.background,
    //     args.window,
    // );

    log::info!("Emulation end");

    Ok(())
}
