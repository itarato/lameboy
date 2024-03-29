use crate::util::*;

macro_rules! make_fn_set_reg_hi {
    ($name:ident, $reg:ident) => {
        pub fn $name(&mut self, byte: u8) {
            self.$reg &= 0b0000_0000_1111_1111;
            self.$reg |= (byte as u16) << 8;
        }
    };
}

macro_rules! make_fn_set_reg_lo {
    ($name:ident, $reg:ident) => {
        pub fn $name(&mut self, byte: u8) {
            self.$reg &= 0b1111_1111_0000_0000;
            self.$reg |= byte as u16;
        }
    };
}

macro_rules! make_fn_get_reg_hi {
    ($name:ident, $reg:ident) => {
        pub fn $name(&self) -> u8 {
            (self.$reg >> 8) as u8
        }
    };
}

macro_rules! make_fn_get_reg_lo {
    ($name:ident, $reg:ident) => {
        pub fn $name(&self) -> u8 {
            (self.$reg & 0xFF) as u8
        }
    };
}

macro_rules! make_fn_get_flag {
    ($name:ident, $offs:expr) => {
        pub fn $name(&self) -> u8 {
            if (self.af & (1 << $offs)) > 0 {
                1
            } else {
                0
            }
        }
    };
}

macro_rules! make_fn_is_flag {
    ($name:ident, $offs:expr) => {
        pub fn $name(&self) -> bool {
            (self.af & (1 << $offs)) > 0
        }
    };
}

macro_rules! make_fn_set_flag {
    ($name:ident, $offs:expr) => {
        pub fn $name(&mut self, is_on: bool) {
            let v = if is_on { 1 } else { 0 };
            assert!(v <= 0b1);
            self.af &= 0xFFFF ^ (1 << $offs);
            self.af |= (v << $offs) as u16;
        }
    };
}

#[derive(Clone, Copy)]
pub enum Reg {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}

pub struct Cpu {
    pub af: u16,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub sp: u16,
    pub pc: u16,
    pub mcycle: u64,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            af: 0,
            bc: 0,
            de: 0,
            hl: 0,
            sp: 0,
            pc: 0,
            // To accomodate mem-read/exec 1-mcycle overlap ignore.
            mcycle: 1,
        }
    }

    make_fn_is_flag!(is_fz, 7);
    make_fn_is_flag!(is_fn, 6);
    make_fn_is_flag!(is_fh, 5);
    make_fn_is_flag!(is_fc, 4);

    make_fn_get_flag!(get_fz, 7);
    make_fn_get_flag!(get_fn, 6);
    make_fn_get_flag!(get_fh, 5);
    make_fn_get_flag!(get_fc, 4);

    make_fn_set_flag!(set_fz, 7);
    make_fn_set_flag!(set_fn, 6);
    make_fn_set_flag!(set_fh, 5);
    make_fn_set_flag!(set_fc, 4);

    make_fn_set_reg_hi!(set_a, af);
    make_fn_set_reg_hi!(set_b, bc);
    make_fn_set_reg_hi!(set_d, de);
    make_fn_set_reg_hi!(set_h, hl);

    // make_fn_set_reg_lo!(set_f, af);
    make_fn_set_reg_lo!(set_c, bc);
    make_fn_set_reg_lo!(set_e, de);
    make_fn_set_reg_lo!(set_l, hl);

    make_fn_get_reg_hi!(get_a, af);
    make_fn_get_reg_hi!(get_b, bc);
    make_fn_get_reg_hi!(get_d, de);
    make_fn_get_reg_hi!(get_h, hl);

    make_fn_get_reg_lo!(get_f, af);
    make_fn_get_reg_lo!(get_c, bc);
    make_fn_get_reg_lo!(get_e, de);
    make_fn_get_reg_lo!(get_l, hl);

    pub fn add(&mut self, add: u8) {
        let is_half_carry = is_half_carry_add_u8(self.get_a(), add);
        let is_carry = is_carry_add_u8(self.get_a(), add);

        let byte = self.get_a().wrapping_add(add);

        self.set_a(byte);

        self.set_fz(byte == 0);
        self.set_fn(false);
        self.set_fh(is_half_carry);
        self.set_fc(is_carry);
    }

    pub fn add_with_carry(&mut self, add: u8) {
        let carry = self.get_fc();

        let is_half_carry = ((self.get_a() & 0xF) + (add & 0xF) + carry) > 0xF;
        let is_carry = self.get_a() as u16 + add as u16 + carry as u16 > 0xffu16;

        let byte = self.get_a().wrapping_add(add).wrapping_add(carry);

        self.set_a(byte);

        self.set_fz(byte == 0);
        self.set_fn(false);
        self.set_fh(is_half_carry);
        self.set_fc(is_carry);
    }

    pub fn cp(&mut self, sub: u8) -> u8 {
        let is_half_carry = is_half_carry_sub_u8(self.get_a(), sub);
        let is_carry = is_carry_sub_u8(self.get_a(), sub);
        let is_zero = self.get_a() == sub;

        self.set_fz(is_zero);
        self.set_fn(true);
        self.set_fh(is_half_carry);
        self.set_fc(is_carry);

        self.get_a().wrapping_sub(sub)
    }

    pub fn sub(&mut self, sub: u8) {
        let byte = self.cp(sub);
        self.set_a(byte);
    }

    pub fn sub_with_carry(&mut self, sub: u8) {
        let is_half_carry = is_half_carry_sub_with_carry_u8(self.get_a(), sub, self.get_fc());
        let is_carry = is_carry_sub_with_carry_u8(self.get_a(), sub, self.get_fc());

        let byte = self.get_a().wrapping_sub(sub).wrapping_sub(self.get_fc());

        self.set_fz(byte == 0);
        self.set_fn(true);
        self.set_fh(is_half_carry);
        self.set_fc(is_carry);

        self.set_a(byte);
    }

    pub fn and(&mut self, and: u8) {
        let byte = self.get_a() & and;
        self.set_a(byte);

        self.set_fz(byte == 0);
        self.set_fn(false);
        self.set_fh(true);
        self.set_fc(false);
    }

    pub fn or(&mut self, or: u8) {
        let byte = self.get_a() | or;
        self.set_a(byte);

        self.set_fz(byte == 0);
        self.set_fn(false);
        self.set_fh(false);
        self.set_fc(false);
    }

    pub fn xor(&mut self, xor: u8) {
        let byte = self.get_a() ^ xor;
        self.set_a(byte);

        self.set_fz(byte == 0);
        self.set_fn(false);
        self.set_fh(false);
        self.set_fc(false);
    }

    pub fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.af &= 0b1111_1111_0000_1111;
        let mut mask = 0u16;

        if z {
            mask |= 0b1000_0000;
        }
        if n {
            mask |= 0b0100_0000;
        }
        if h {
            mask |= 0b0010_0000;
        }
        if c {
            mask |= 0b0001_0000;
        }

        self.af |= mask;
    }

    pub fn shift_left_instrucrtion(&mut self, reg: Reg) {
        let is_carry = is_carry_rot_left_u8(self.get_reg(reg));
        let byte = (self.get_reg(reg) << 1) | self.get_fc();

        self.set_reg(reg, byte);
        self.set_flags(byte == 0, false, false, is_carry);
    }

    pub fn shift_right_instruction(&mut self, reg: Reg) {
        let is_carry = is_carry_rot_right_u8(self.get_reg(reg));
        let byte = (self.get_reg(reg) >> 1) | (self.get_fc() << 7);

        self.set_reg(reg, byte);
        self.set_flags(byte == 0, false, false, is_carry);
    }

    fn get_reg(&self, reg: Reg) -> u8 {
        match reg {
            Reg::A => self.get_a(),
            Reg::B => self.get_b(),
            Reg::C => self.get_c(),
            Reg::D => self.get_d(),
            Reg::E => self.get_e(),
            Reg::H => self.get_h(),
            Reg::L => self.get_l(),
        }
    }

    fn set_reg(&mut self, reg: Reg, value: u8) {
        match reg {
            Reg::A => self.set_a(value),
            Reg::B => self.set_b(value),
            Reg::C => self.set_c(value),
            Reg::D => self.set_d(value),
            Reg::E => self.set_e(value),
            Reg::H => self.set_h(value),
            Reg::L => self.set_l(value),
        }
    }
}
