use std::collections::VecDeque;

pub fn is_carry_add_u8(acc: u8, add: u8) -> bool {
    (u8::MAX - acc) < add
}

pub fn is_carry_add_u16(acc: u16, add: u16) -> bool {
    (u16::MAX - acc) < add
}

pub fn is_carry_sub_u8(acc: u8, sub: u8) -> bool {
    acc < sub
}

// pub fn is_carry_sub_u16(acc: u16, sub: u16) -> bool {
//     acc < sub
// }

pub fn is_carry_rot_left_u8(acc: u8) -> bool {
    is_bit(acc, 7)
}

pub fn is_carry_rot_right_u8(acc: u8) -> bool {
    is_bit(acc, 0)
}

pub fn is_carry_shift_left_u8(acc: u8) -> bool {
    is_carry_rot_left_u8(acc)
}

pub fn is_carry_shift_right_u8(acc: u8) -> bool {
    is_carry_rot_right_u8(acc)
}

pub fn is_half_carry_add_u8(acc: u8, n: u8) -> bool {
    ((acc & 0xF) + (n & 0xF)) & 0x10 == 0x10
}

pub fn is_half_carry_add_u16(acc: u16, n: u16) -> bool {
    ((acc & 0xFFF) + (n as u16 & 0xFFF)) & 0x1000 == 0x1000
}

pub fn is_half_carry_sub_u8(acc: u8, sub: u8) -> bool {
    // TODO: WHICH ONE IS IT???
    // (acc & 0x0F) < (sub & 0x0F)
    // (acc & 0xF0).wrapping_sub(sub & 0xF0) & 0x8 == 0x8
    // (acc as i16 & 0xf) - (sub as i16 & 0xf) < 0
    (acc & 0xf) > 0
}

pub fn shift_left_a(byte: u8) -> u8 {
    (byte as i8).wrapping_shl(1) as u8
}

pub fn shift_right_logical(byte: u8) -> u8 {
    byte >> 1
}

pub fn wrapping_add_u16_i8(lhs: u16, rhs: i8) -> u16 {
    if rhs >= 0 {
        lhs.wrapping_add(rhs as u16)
    } else {
        lhs.wrapping_sub(rhs.abs() as u16)
    }
}

pub fn shift_right_arithmetic_u8(byte: u8) -> u8 {
    (byte >> 1) | (byte & 0b1000_0000)
}

pub fn swap(byte: u8) -> u8 {
    (byte << 4) | (byte >> 4)
}

pub fn bit(byte: u8, n: u8) -> u8 {
    (byte >> n) & 0b1
}

pub fn is_bit(byte: u8, n: u8) -> bool {
    bit(byte, n) > 0
}

pub fn set_bit(mut byte: u8, n: u8, is_on: bool) -> u8 {
    byte &= !(1 << n);
    if is_on {
        byte |= 1 << n;
    }
    byte
}

pub struct SizedQueue<T> {
    capacity: usize,
    deque: VecDeque<T>,
}

impl<T> SizedQueue<T> {
    pub fn new(capacity: usize) -> SizedQueue<T> {
        SizedQueue {
            deque: VecDeque::new(),
            capacity,
        }
    }

    pub fn push(&mut self, e: T) {
        while self.deque.len() >= self.capacity {
            self.deque.pop_front();
        }

        self.deque.push_back(e);
    }

    pub fn inner(&self) -> &VecDeque<T> {
        &self.deque
    }
}

#[cfg(test)]
mod tests {
    use crate::util::*;

    #[test]
    fn test_is_carry_rot_left_u8() {
        assert!(is_carry_rot_left_u8(0b1100_0000));
        assert!(!is_carry_rot_left_u8(0b0100_0000));
        assert!(is_carry_rot_right_u8(0b0100_0001));
        assert!(!is_carry_rot_right_u8(0b0100_0010));
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
