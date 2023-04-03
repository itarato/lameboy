mod cartridge;
mod conf;
mod cpu;
mod debugger;
mod gfx;
mod mem;
mod sound;
mod timer;
mod util;
mod video;
mod vm;

use std::sync::Arc;
use std::sync::Mutex;

use crate::cartridge::Cartridge;
use crate::conf::*;
use crate::debugger::*;
use crate::vm::*;

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
    simple_logger::init_with_env().unwrap();
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
    let vram = Arc::new(Mutex::new([0; VRAM_SIZE]));
    let oam_ram = Arc::new(Mutex::new([0; OAM_RAM_SIZE]));
    let mut vm = VM::new(cartridge, vram.clone(), oam_ram.clone(), debugger)?;

    vm.setup()?;
    vm.run()?;

    log::info!("Emulation end");

    Ok(())
}
