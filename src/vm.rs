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
