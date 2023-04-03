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

use crate::cartridge::*;
use crate::conf::*;
use crate::debugger::*;
use crate::gfx::*;
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

    let vm_vram = vram.clone();
    let vm_oam_ram = oam_ram.clone();
    let vm_thread = spawn(|| {
        if let Ok(mut vm) = VM::new(cartridge, vm_vram, vm_oam_ram, debugger) {
            if let Err(err) = vm.setup() {
                log::error!("Failed VM setup: {}", err);
                return;
            }

            if let Err(err) = vm.run() {
                log::error!("Failed VM run: {}", err);
                return;
            }
        }
    });

    let gfx_vram = vram.clone();
    let gfx_oam_ram = oam_ram.clone();
    let gfx_thread = spawn(|| {
        let gfx = Gfx::new(gfx_vram, gfx_oam_ram);
        gfx.run();
    });

    gfx_thread.join().expect("Failed joining GFX thread");
    vm_thread.join().expect("Failed joining VM thread");

    log::info!("Emulation end");

    Ok(())
}
