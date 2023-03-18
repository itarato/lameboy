pub fn is_carry_add_u8(acc: u8, add: u8) -> bool {
    (u8::MAX - acc) < add
}

pub fn is_carry_add_u16(acc: u16, add: u16) -> bool {
    (u16::MAX - acc) < add
}

pub fn is_carry_sub_u8(acc: u8, sub: u8) -> bool {
    acc < sub
}

pub fn is_carry_sub_u16(acc: u16, sub: u16) -> bool {
    acc < sub
}

pub fn is_carry_rot_left_u8(acc: u8, n: u8) -> bool {
    (acc >> (8 - n)) > 0
}

pub fn is_carry_rot_right_u8(acc: u8, n: u8) -> bool {
    (acc << (8 - n)) > 0
}

pub fn is_carry_shift_left_u8(acc: u8, n: u8) -> bool {
    is_carry_rot_left_u8(acc, n)
}

pub fn is_carry_shift_right_u8(acc: u8, n: u8) -> bool {
    is_carry_rot_right_u8(acc, n)
}

pub fn is_half_carry_add_u8(acc: u8, n: u8) -> bool {
    (acc & 0xF) + (n & 0xF) > 0xF
}

pub fn is_half_carry_add_u16(acc: u16, n: u16) -> bool {
    (acc & 0xFFF) + (n as u16 & 0xFFF) > 0xFFF
}

pub fn is_half_carry_sub_u8(acc: u8, sub: u8) -> bool {
    (acc & 0xF0) < (sub & 0xF0)
}

pub fn is_half_carry_sub_u16(acc: u16, sub: u16) -> bool {
    (acc & 0xF000) < (sub & 0xF000)
}

pub fn shift_left_a(byte: u8) -> u8 {
    (byte as i8).wrapping_shl(1) as u8
}

pub fn shift_right_a(byte: u8) -> u8 {
    (byte as i8).wrapping_shr(1) as u8
}

pub fn shift_left_l(byte: u8) -> u8 {
    byte << 1
}

pub fn shift_right_l(byte: u8) -> u8 {
    byte >> 1
}

pub fn wrapping_add_u16_i8(lhs: u16, rhs: i8) -> u16 {
    if rhs >= 0 {
        lhs.wrapping_add(rhs as u16)
    } else {
        lhs.wrapping_sub(rhs.abs() as u16)
    }
}

pub fn swap(byte: u8) -> u8 {
    (byte << 4) | (byte >> 4)
}

pub fn is_bit(byte: u8, n: u8) -> bool {
    ((byte >> n) & 0b1) > 0
}

pub fn set_bit(mut byte: u8, n: u8, is_on: bool) -> u8 {
    byte &= !(1 << n);
    if is_on {
        byte |= 1 << n;
    }
    byte
}

#[cfg(test)]
mod tests {
    use crate::util::*;

    #[test]
    fn test_is_carry_rot_left_u8() {
        assert!(is_carry_rot_left_u8(0b1100_0000, 1));

        assert!(!is_carry_rot_left_u8(0b0100_0000, 1));
        assert!(is_carry_rot_left_u8(0b0100_0000, 2));
        assert!(is_carry_rot_left_u8(0b0100_0000, 3));
        assert!(is_carry_rot_left_u8(0b0100_0000, 4));
        assert!(is_carry_rot_left_u8(0b0100_0000, 5));
        assert!(is_carry_rot_left_u8(0b0100_0000, 6));
        assert!(is_carry_rot_left_u8(0b0100_0000, 7));

        assert!(is_carry_rot_right_u8(0b0100_0001, 1));

        assert!(!is_carry_rot_right_u8(0b0100_0010, 1));
        assert!(is_carry_rot_right_u8(0b0100_0010, 2));
        assert!(is_carry_rot_right_u8(0b0100_0010, 3));
        assert!(is_carry_rot_right_u8(0b0100_0010, 4));
        assert!(is_carry_rot_right_u8(0b0100_0010, 5));
        assert!(is_carry_rot_right_u8(0b0100_0010, 6));
        assert!(is_carry_rot_right_u8(0b0100_0010, 7));
    }

    #[test]
    fn test_swap() {
        assert_eq!(0b0110_1001, swap(0b1001_0110));
    }

    #[test]
    fn test_is_bit() {
        assert!(is_bit(0b0000_1000, 3));
        assert!(!is_bit(0b0000_1000, 4));
        assert!(!is_bit(0b0000_1000, 2));
    }
}
