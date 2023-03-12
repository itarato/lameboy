pub fn is_carry_add_u8(acc: u8, add: u8) -> bool {
    (u8::MAX - acc) < add
}

pub fn is_carry_add_u16(acc: u16, add: u8) -> bool {
    (u16::MAX - acc) < add as u16
}

pub fn is_carry_sub_u8(acc: u8, sub: u8) -> bool {
    acc < sub
}

pub fn is_carry_rot_left_u8(acc: u8, n: u8) -> bool {
    (acc >> (8 - n)) > 0
}

pub fn is_carry_rot_right_u8(acc: u8, n: u8) -> bool {
    (acc << (8 - n)) > 0
}

pub fn is_half_carry_add_u8(acc: u8, n: u8) -> bool {
    (acc & 0xF) + (n & 0xF) > 0xF
}

pub fn is_half_carry_add_u16(acc: u16, n: u8) -> bool {
    (acc & 0xF) + (n as u16 & 0xF) > 0xF
}
