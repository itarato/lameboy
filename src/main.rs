mod cartridge;
mod conf;
mod cpu;
mod debugger;
mod gfx;
mod joypad;
mod mem;
mod serial;
mod sound;
mod timer;
mod util;
mod video;
mod vm;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
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
    breakpoint: Option<String>,

    /// Step by step.
    #[arg(short = 's', long)]
    step_by_step: bool,

    // Skip FPS limiter.
    #[arg(short, long)]
    nofps: bool,

    // Dump opcode list to file.
    #[arg(long)]
    dump: bool,
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

    let breakpoint_flag = Arc::new(AtomicBool::new(false));
    let mut debugger = Debugger::new(breakpoint_flag.clone());
    if args.debug {
        args.breakpoint_parsed()
            .map(|breakpoint| debugger.add_breakpoint(breakpoint));
        if args.step_by_step {
            debugger.set_break_on_start();
            debugger.set_step_by_step();
        }
    }

    let cartridge = Cartridge::new(args.cartridge)?;
    let global_exit_flag = Arc::new(AtomicBool::new(false));
    let video = Arc::new(RwLock::new(Video::new(args.nofps)));
    let is_opcode_dump = args.dump;

    let vm_thread = spawn({
        let global_exit_flag = global_exit_flag.clone();
        let video = video.clone();

        move || {
            if let Ok(mut vm) = VM::new(
                global_exit_flag.clone(),
                cartridge,
                debugger,
                video,
                is_opcode_dump,
            ) {
                if let Err(err) = vm.setup() {
                    log::error!("Failed VM setup: {}", err);
                    global_exit_flag.store(true, std::sync::atomic::Ordering::Release);
                    return;
                }

                if let Err(err) = vm.run() {
                    log::error!("Failed VM run: {}", err);
                    vm.dump_op_history();
                    global_exit_flag.store(true, std::sync::atomic::Ordering::Release);
                    return;
                }
            }

            global_exit_flag.store(true, std::sync::atomic::Ordering::Release);
        }
    });

    let gfx = Gfx::new(global_exit_flag.clone(), video.clone());
    gfx.run(breakpoint_flag);

    global_exit_flag.store(true, std::sync::atomic::Ordering::Release);

    vm_thread.join().expect("Failed joining VM thread");

    log::info!("Emulation end");

    Ok(())
}
