use bitvec::prelude::*;
use byteorder::{ByteOrder, BigEndian, ReadBytesExt};
use std::io::Cursor;

#[derive(Clone, PartialEq, PartialOrd)]
// where msb0 is big endian.
pub struct Exponent {
    vec: BitVec<Msb0>,
}
/// Exponent is a 14-bit portion of decimal128 that follows the sign bit. Here we
/// are storing it as a 16-bit BitVec that can be later converted to a u16.
impl Exponent {
    pub fn new() -> Self {
        Exponent {
            vec: bitvec![Msb0, u8; 0; 2],
        }
    }

    pub fn append(&mut self, vec: &mut BitVec) {
        self.vec.append(vec)
    }

    pub fn is_zero(&self) -> bool {
        self.to_num() == 0
    }

    pub fn to_num(&self) -> u16 {
        let mut reader = Cursor::new(&self.vec);
        reader.read_u16::<BigEndian>().unwrap()
    }

    // compare current exponent value with exponent bias (largest possible
    // exponent value)
    // TODO: check if 6176 (exponent bias) can be stored as u16
    pub fn to_adjusted(&self) -> i16 {
        self.to_num() as i16 - 6176 as i16
    }
}