mod cartridge;
mod conf;
mod cpu;
mod debugger;
mod gfx;
mod mem;
mod serial;
mod sound;
mod timer;
mod util;
mod video;
mod vm;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use crate::cartridge::*;
use crate::conf::*;
use crate::debugger::*;
use crate::gfx::*;
use crate::video::Video;
use crate::vm::*;

use std::thread::spawn;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Cartridge.
    cartridge: String,

    /// Enable debug mode.
    #[arg(short, long)]
    debug: bool,

    /// Breakpoints.
    #[arg(short = 'b', long)]
    breakpoints: Vec<u16>,

    /// Step by step.
    #[arg(short = 's', long)]
    step_by_step: bool,
}

fn main() -> Result<(), Error> {
    simple_logger::SimpleLogger::new()
        .env()
        .with_module_level("wgpu_core", log::LevelFilter::Off)
        .init()
        .unwrap();
    log::info!("Emulation start");

    let args = Args::parse();

    let mut debugger = Debugger::new();
    if args.debug {
        debugger.set_break_on_start();
        debugger.add_breakpoints(args.breakpoints);
        if args.step_by_step {
            debugger.set_step_by_step();
        }
    }

    let cartridge = Cartridge::new(args.cartridge)?;
    let wram = Arc::new(Mutex::new([0; WRAM_SIZE]));
    let canvas = Arc::new(Mutex::new([0; DISPLAY_PIXELS_COUNT as usize]));
    let global_exit_flag = Arc::new(AtomicBool::new(false));
    let video = Arc::new(RwLock::new(Video::new()));

    let vm_thread = spawn({
        let wram = wram.clone();
        let global_exit_flag = global_exit_flag.clone();
        let video = video.clone();

        move || {
            if let Ok(mut vm) = VM::new(global_exit_flag.clone(), cartridge, debugger, video) {
                if let Err(err) = vm.setup() {
                    log::error!("Failed VM setup: {}", err);
                    global_exit_flag.store(true, std::sync::atomic::Ordering::Release);
                    return;
                }

                if let Err(err) = vm.run() {
                    log::error!("Failed VM run: {}", err);
                    global_exit_flag.store(true, std::sync::atomic::Ordering::Release);
                    return;
                }
            }

            global_exit_flag.store(true, std::sync::atomic::Ordering::Release);
        }
    });

    let gfx = Gfx::new(global_exit_flag.clone(), video.clone());
    gfx.run();

    global_exit_flag.store(true, std::sync::atomic::Ordering::Release);

    vm_thread.join().expect("Failed joining VM thread");

    log::info!("Emulation end");

    Ok(())
}
