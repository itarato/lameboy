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

pub fn is_carry_sub_with_carry_u8(acc: u8, sub: u8, carry: u8) -> bool {
    (acc as u16) < sub as u16 + carry as u16
}

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
    // TODO: Reaaaaaly not sure about this.
    (acc & 0x0F) < (sub & 0x0F)
}

pub fn is_half_carry_sub_with_carry_u8(acc: u8, sub: u8, carry: u8) -> bool {
    // TODO: Reaaaaaly not sure about this.
    (acc & 0x0F) < (sub & 0x0F) + carry
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

#[derive(Debug)]
pub struct Counter {
    counter: u64,
    modulo: u64,
}

impl Counter {
    pub fn new(modulo: u64) -> Counter {
        Counter { counter: 0, modulo }
    }

    pub fn tick_and_check_overflow(&mut self, len: u64) -> bool {
        self.tick(len);
        self.check_overflow()
    }

    pub fn tick(&mut self, len: u64) {
        self.counter += len;
    }

    pub fn check_overflow(&mut self) -> bool {
        if self.counter >= self.modulo {
            self.counter -= self.modulo;
            assert!(self.counter < self.modulo);
            true
        } else {
            false
        }
    }

    pub fn check_overflow_count(&mut self) -> u8 {
        let mut count = 0;

        while self.counter >= self.modulo {
            self.counter -= self.modulo;
            count += 1;
        }

        count
    }

    pub fn update_modulo(&mut self, modulo: u64) {
        self.modulo = modulo;
    }

    pub fn reset(&mut self) {
        self.counter = 0;
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
