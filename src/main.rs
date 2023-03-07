mod conf;
mod cpu;
mod mem;
mod vm;

use crate::conf::Error;
use crate::vm::*;

fn main() -> Result<(), Error> {
    let mut vm = VM::new()?;
    vm.run()?;

    Ok(())
}
