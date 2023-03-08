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
        pub fn $name(&mut self) -> u8 {
            (self.$reg >> 8) as u8
        }
    };
}

macro_rules! make_fn_get_reg_lo {
    ($name:ident, $reg:ident) => {
        pub fn $name(&mut self) -> u8 {
            (self.$reg & 0xFF) as u8
        }
    };
}

pub struct Cpu {
    af: u16,
    bc: u16,
    de: u16,
    pub hl: u16,
    sp: u16,
    pub pc: u16,
    pub mcycle: usize,
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

    make_fn_set_reg_hi!(set_a, af);
    make_fn_set_reg_hi!(set_b, bc);
    make_fn_set_reg_hi!(set_d, de);
    make_fn_set_reg_hi!(set_h, hl);

    make_fn_set_reg_lo!(set_f, af);
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
}
