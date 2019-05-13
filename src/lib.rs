//! Decimal 128 bits are broken down like so:
//! [1bits]  [   14bits   ]  [   113 bits   ]
//!  sign       exponent        significand
//!              field  
use bitvec::{bitvec, BitVec, LittleEndian};
use byteorder::*;
use failure::ensure;
use std::io::Cursor;

#[derive(Debug, Clone, PartialOrd)]
pub struct Exponent {
    vec: BitVec<LittleEndian>,
}
#[derive(Debug, Clone, PartialOrd)]
pub struct Significand {
    vec: BitVec<LittleEndian>,
}

pub struct Decimal128 {
    sign: bool,
    exponent: Option<Exponent>,
    significand: Option<Significand>,
}

pub enum CombinationField {
    NaN,
    Infinity,
    Finite,
}

impl Decimal128 {
    /// Create a Decimal128 from a &[u8; 16]. Will panic if the vector used is
    /// the wrong length.
    ///
    /// This method extracts out the sign, exponent and signficand, uses Binary
    /// Integer Decimal decoding. The byte order is LittleEndian. For more
    /// information on how extraction is done, please refer to
    /// [wikipedia](https://en.wikipedia.org/wiki/Decimal128_floating-point_format),
    /// or the [IEEE 754-2008](https://ieeexplore.ieee.org/document/4610935)
    /// ```
    /// use decimal128::*;
    ///
    /// let vec = vec![9, 16, 3, 6, 7, 86, 76, 81, 89, 0, 3, 45, 12, 71, 52, 39];
    /// let dec128 = Decimal128::from_raw_buf(&vec).unwrap();
    /// ```
    pub fn from_raw_buf(buffer: &[u8]) -> Result<Self, failure::Error> {
        ensure!(buffer.len() == 16, "buffer should be 16 bytes");
        // decimal 128's exponent is 14bits long; we will construct a u16 and
        // fill up the first two bits as zeros and then get its value.
        let mut total_exp = Exponent::new();
        // Significnad can be 113 *or* 111 bit long. Regardless of the size we
        // will pad it with 14 0s. We will be eventually constructing a u128
        // from this eventually.
        let mut total_sig = Significand::new();

        let byte = buffer[0];
        let max = 0b1111_1111;
        // first bit is sign: negative or positive integer
        let is_negative_bitmask = 0b0111_1111;
        let sign = (byte | is_negative_bitmask) == max;

        // the next 5 bits of the first byte are combination field; these include:
        // first 5 bits       Type	    Exponent MSBs	Significand MSD
        // ---------------------------------------------------------------------------
        //     a b c d e	  Finite       14bits           113bits
        //     1 1 c d e	  Finite    2 bits to right     111bits
        //     1 1 1 1 0	  Infinity	    - -	            - - - -
        //     1 1 1 1 1	  NaN           - -             - - - -
        // the easiest bitmask to do is for NaN, i.e. five 1s
        let res = byte | 0b1000_0011;
        let combination_field = match res {
            // if everything is 1s, we are looking at NaN
            0b1111_1111 => CombinationField::NaN,
            // if the last of the five bits is a 0, we are looking at Infinity
            0b1111_1011 => CombinationField::Infinity,
            // match for finite cases
            _ => match byte | 0b1001_1111 {
                0b1111_1111 => {
                    // since the first two bits after the sign are 11, we ignore
                    // them and gather the remainder of the first byte.
                    let c = if (byte | 0b1110_1111) == max { 1 } else { 0 };
                    let d = if (byte | 0b1111_0111) == max { 1 } else { 0 };
                    let e = if (byte | 0b1111_1011) == max { 1 } else { 0 };
                    let f = if (byte | 0b1111_1101) == max { 1 } else { 0 };
                    let g = if (byte | 0b1111_1110) == max { 1 } else { 0 };
                    let mut exp = bitvec![c, d, e, f, g];
                    total_exp.append(&mut exp);
                    // in this case second byte of the buffer can just be
                    // straight up appended to the exponent.
                    let byte_2 = buffer[1];
                    let mut sb_bv: BitVec = (&[byte_2] as &[u8]).into();
                    total_exp.append(&mut sb_bv);
                    // out of the third byte the first bit are part of the
                    // exponent, and the last 7 bits are part of the significand
                    let byte_3 = buffer[1];
                    let h = if (byte_2 | 0b0111_1111) == max { 1 } else { 0 };
                    let mut exp_cont = bitvec![h];
                    total_exp.append(&mut exp_cont);
                    let i = if (byte_3 | 0b1011_1111) == max { 1 } else { 0 };
                    let j = if (byte_3 | 0b1101_1111) == max { 1 } else { 0 };
                    let k = if (byte_3 | 0b1110_1111) == max { 1 } else { 0 };
                    let l = if (byte_3 | 0b1111_0111) == max { 1 } else { 0 };
                    let m = if (byte_3 | 0b1111_1011) == max { 1 } else { 0 };
                    let n = if (byte_3 | 0b1111_1101) == max { 1 } else { 0 };
                    let o = if (byte_3 | 0b1111_1110) == max { 1 } else { 0 };
                    // Start a new vec for 111bit significand. This version of
                    // the significand is offset by two bits, so we pad it with
                    // `100`
                    let mut sig = bitvec![1, 0, 0, i, j, k, l, m, n, o];
                    total_sig.append(&mut sig);
                    CombinationField::Finite
                }
                _ => {
                    // if the first two bits after the sign are `00`, `01`,
                    // `10`, we add the remainder of the first byte to exponent
                    let a = if (byte | 0b1011_1111) == max { 1 } else { 0 };
                    let b = if (byte | 0b1101_1111) == max { 1 } else { 0 };
                    let c = if (byte | 0b1110_1111) == max { 1 } else { 0 };
                    let d = if (byte | 0b1111_0111) == max { 1 } else { 0 };
                    let e = if (byte | 0b1111_1011) == max { 1 } else { 0 };
                    let f = if (byte | 0b1111_1101) == max { 1 } else { 0 };
                    let g = if (byte | 0b1111_1110) == max { 1 } else { 0 };
                    let mut exp = bitvec![a, b, c, d, e, f, g];
                    total_exp.append(&mut exp);
                    // out of the second byte the first 7 bits are part of the
                    // exponent, and the last bit if part of the significand
                    let byte_2 = buffer[1];
                    let h = if (byte_2 | 0b0111_1111) == max { 1 } else { 0 };
                    let i = if (byte_2 | 0b1011_1111) == max { 1 } else { 0 };
                    let j = if (byte_2 | 0b1101_1111) == max { 1 } else { 0 };
                    let k = if (byte_2 | 0b1110_1111) == max { 1 } else { 0 };
                    let l = if (byte_2 | 0b1111_0111) == max { 1 } else { 0 };
                    let m = if (byte_2 | 0b1111_1011) == max { 1 } else { 0 };
                    let n = if (byte_2 | 0b1111_1101) == max { 1 } else { 0 };
                    let mut exp_cont = bitvec![h, i, j, k, l, m, n];
                    total_exp.append(&mut exp_cont);
                    let o = if (byte_2 | 0b1111_1110) == max { 1 } else { 0 };
                    // Start a new vec for 113bit significand. Since this
                    // version of significand is not offset, we pad it with only
                    // `0`
                    let mut sig = bitvec![0, o];
                    total_sig.append(&mut sig);
                    // add the whole third byte to the signficand in this case
                    let byte_3 = buffer[2];
                    let mut tb_bv: BitVec = (&[byte_3] as &[u8]).into();
                    total_sig.append(&mut tb_bv);
                    CombinationField::Finite
                }
            },
        };

        // the rest of the bytes of the vec we are passed in.
        for bytes in 3..buffer.len() {
            let mut bv: BitVec = (&[buffer[bytes]] as &[u8]).into();
            total_sig.append(&mut bv);
        }

        let dec128 = match combination_field {
            CombinationField::Finite => Decimal128 {
                sign,
                exponent: Some(total_exp),
                significand: Some(total_sig),
            },
            _ => Decimal128 {
                sign,
                exponent: None,
                significand: None,
            },
        };
        Ok(dec128)
    }

    /// Converts Decimal128 to string. Uses information in
    /// [speleotrove](http://speleotrove.com/decimal/daconvs.html) decimal
    /// documentation.
    pub fn to_string(&self) -> Option<String> {
        if self.exponent?.to_adjusted_exponent() > 0 {
            if self.significand?.to_num() > exponent
        }
        unimplemented!()
    }

    pub fn use_scientific_notation(&self) -> bool {
        self.exponent?.to_adjusted_exponent() > 0 || scientific_exponent < -6
    }

    // TODO: check if we can just return a number here
    // TODO: match up number types with significand and exponenet
    pub fn scientific_exponent(&self) -> Option<u128> {
        (self.significand?.to_num().len() - 1) + self.exponent?.to_adjusted_exponent()
    }
}

/// Exponent is a 14-bit portion of decimal128 that follows the sign bit. Here we
/// are storing it as a 16-bit BitVec that can be later converted to a u16.
impl Exponent {
    pub fn new() -> Self {
        Exponent {
            vec: bitvec![LittleEndian, u8; 0; 2],
        }
    }

    pub fn append(&mut self, vec: &mut BitVec) {
        self.vec.append(vec)
    }

    pub fn to_num(&self) -> u16 {
        let mut reader = Cursor::new(&self.vec);
        reader.read_u16::<byteorder::LittleEndian>().unwrap()
    }

    // compare current exponent value with exponent bias (largest possible
    // exponent value)
    // TODO: check if 6176 (exponent bias) can be stored as u16
    pub fn to_adjusted_exponent(&self) -> u16 {
        &self.to_num() - 6176
    }
}

/// Significand is a padded 111- or 113-bit coefficient. We are storing it as a
/// 128-bit BitVec with the padded difference. This can be converted to a u128.
impl Significand {
    pub fn new() -> Self {
        Significand {
            vec: bitvec![LittleEndian, u8; 0; 14],
        }
    }

    pub fn append(&mut self, vec: &mut BitVec) {
        self.vec.append(vec)
    }

    pub fn to_num(&self) -> u128 {
        let mut reader = Cursor::new(&self.vec);
        reader.read_u128::<byteorder::LittleEndian>().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let vec = vec![9, 16, 3, 6, 7, 86, 76, 81, 89, 0, 3, 45, 12, 71, 52, 39];
        let dec128 = Decimal128::from_raw_buf(&vec).unwrap();
        dec128.to_string();
    }
}
