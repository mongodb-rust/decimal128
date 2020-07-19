use bitvec::prelude::*;
use byteorder::{BigEndian};
use std::io::Cursor;

#[derive(Clone, PartialEq, PartialOrd)]
// where msb0 is big endian.
pub struct Significand {
    vec: BitVec<Msb0>,
}

/// Significand is a padded 111- or 113-bit coefficient. We are storing it as a
/// 128-bit BitVec with the padded difference. This can be converted to a u128.
impl Significand {
    pub fn new() -> Self {
        Significand {
            vec: bitvec![Msb0, u8; 0; 14],
        }
    }

    pub fn append(&mut self, vec: &mut BitVec) {
        self.vec.append(vec)
    }

    pub fn is_zero(&self) -> bool {
        // FIXME: Very inefficient, but docs are down
        self.count_digits() == 0
    }

    pub fn to_num(&self) -> u128 {
        let mut reader = Cursor::new(&self.vec);
        // BigEndian::read_u128(&self.vec)
        reader.read_u128::<BigEndian>().unwrap()
    }

    pub fn max_value() -> u128 {
        u128::from_str_radix("9999999999999999999999999999999999", 10).unwrap()
    }

    // count the number of digits in the significand. This method first converts
    // significand BitVec into a u128 number, then converts it to string to
    // count characters and collects them in a vec to look at the vec's length.
    //
    // We return a u16 number of digits, as it's easier to compare to the
    // exponent since that's also stored as a u16.
    pub fn count_digits(&self) -> i16 {
        self.as_digit_vec().len() as i16
    }

    pub fn as_digit_vec(&self) -> Vec<u32> {
        let digits: Vec<u32> = self
            .to_num()
            .to_string()
            .chars()
            .map(|c| c.to_digit(10).unwrap())
            .collect();
        return digits;
    }
}