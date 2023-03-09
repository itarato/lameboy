use simple_logger::SimpleLogger;

mod cartridge;
mod conf;
mod cpu;
mod mem;
mod util;
mod vm;

use crate::conf::Error;
use crate::vm::*;

fn main() -> Result<(), Error> {
    SimpleLogger::new().init().unwrap();
    log::info!("Emulation start");

    let mut vm = VM::new()?;

    vm.setup()?;
    vm.run()?;

    log::info!("Emulation end");

    Ok(())
}
