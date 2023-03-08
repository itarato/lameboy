use std::fs::File;
use std::io::Read;

use crate::conf::*;
use crate::cpu::*;
use crate::mem::*;

pub struct VM {
    mem: Mem,
    cpu: Cpu,
}

impl VM {
    pub fn new() -> Result<Self, Error> {
        Ok(VM {
            mem: Mem::new()?,
            cpu: Cpu::new(),
        })
    }

    pub fn setup(&mut self) -> Result<(), Error> {
        let bios = &mut self.mem.bios;
        let mut bios_file = File::open("assets/dmg_boot.bin")?;

        let bios_read_len = bios_file.read(bios)?;
        assert_eq!(BIOS_SIZE, bios_read_len, "BIOS read size not match");

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Error> {
        self.reset();

        loop {
            self.exec_opcode();

            unimplemented!()
        }
    }

    fn reset(&mut self) {}

    fn exec_opcode(&mut self) {}
}
