pub fn u8_hi2(byte: u8) -> u8 {
    byte >> 6
}

pub fn u8_lo3(byte: u8) -> u8 {
    byte & 0b111
}

pub fn u8_lo4(byte: u8) -> u8 {
    byte & 0b1111
}
