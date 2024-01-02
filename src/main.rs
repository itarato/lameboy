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

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::RwLock;

use crate::cartridge::*;
use crate::conf::*;
use crate::debugger::*;
use crate::ppu::PPU;
use crate::vm::*;

use std::thread::spawn;

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

    /// VRam debug window.
    #[arg(long)]
    debug_vram: bool,

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

    let breakpoint_flag = Arc::new(AtomicBool::new(false));
    let mut debugger = Debugger::new(breakpoint_flag.clone());

    args.breakpoint_parsed()
        .map(|breakpoint| debugger.add_breakpoint(breakpoint));
    if args.step_by_step {
        debugger.set_break_on_start();
        debugger.set_step_by_step();
    }

    let global_exit_flag = Arc::new(AtomicBool::new(false));
    let video = Arc::new(RwLock::new(PPU::new(args.nofps)));
    let joypad_button_input_requester = Arc::new(RwLock::new(joypad::JoypadInputRequest::new()));
    let joypad = joypad::Joypad::new(joypad_button_input_requester.clone());

    let vm_thread = spawn({
        let global_exit_flag = global_exit_flag.clone();
        let video = video.clone();

        move || {
            if let Ok(mut vm) = VM::new(
                global_exit_flag.clone(),
                Cartridge::new(args.cartridge).expect("Cannot open cartridge"),
                debugger,
                video,
                args.opcode_dump,
                joypad,
                args.disable_sound,
            ) {
                if let Err(err) = vm.setup(args.skip_intro) {
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

    gfx::run(
        global_exit_flag.clone(),
        video.clone(),
        breakpoint_flag,
        joypad_button_input_requester,
        args.debug_vram,
    );

    global_exit_flag.store(true, std::sync::atomic::Ordering::Release);

    vm_thread.join().expect("Failed joining VM thread");

    log::info!("Emulation end");

    Ok(())
}
