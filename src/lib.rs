use bitvec::*;
use byteorder::ReadBytesExt;
use failure::ensure;
use std::io::Cursor;

pub struct Exponent(BitVec);
pub struct Coefficient(BitVec);

pub struct Decimal128 {
    sign: bool,
    exponent: Option<Exponent>,
    coefficient: Option<Coefficient>,
}

pub enum CombinationField {
    NaN,
    Infinity,
    Finite(Exponent, Coefficient),
}

impl Decimal128 {
    /// Decimal 128 bits are broken down like so:
    /// [1bits]  [   5bits   ]  [   12bits   ]   [    110bits    ]
    ///  sign     combination      exponent         coefficient
    ///              field       continuation       continuation
    pub fn from_raw_buf(buffer: &[u8]) -> Result<Decimal128, failure::Error> {
        ensure!(buffer.len() == 16, "buffer should be 16 bytes");
        // decimal 128's exponent is 14bits long; we will construct a u16 and
        // fill up the first two bits as zeros and then get its value
        let mut total_exp: BitVec = bitvec![BigEndian, u8; 0; 2];

        let mut rdr = Cursor::new(buffer);
        let byte = rdr.read_u8().unwrap();
        let max = 0b11111111;
        // first bit is sign: negative or positive integer
        let is_negative_bitmask = 0b01111111;
        let sign = (byte | is_negative_bitmask) == max;

        // the next 5 bits of the first byte are combination field; these include:
        // Combination field	Type	Exponent MSBs	Coefficient MSD
        //      (5 bits)                  (2 bits)        (4 bits)
        // ---------------------------------------------------------------------------
        //     a b c d e	  Finite        a b	          0 c d e
        //     1 1 c d e	  Finite        c d	          1 0 0 e
        //     1 1 1 1 0	  Infinity	    - -	          - - - -
        //     1 1 1 1 1	  NaN           - -           - - - -
        let combination_bitmask = 0b10000011;
        let res = byte | combination_bitmask;
        let combination_field = match res {
            // if everything is 1s, we are looking at NaN
            0b11111111 => CombinationField::NaN,
            // if the last of the five bits is a 0, we are looking at Infinity
            0b11111011 => CombinationField::Infinity,
            // match for finite cases to get exponent MSBs and coefficient MSDs
            _ => match byte | 0b1001111 {
                0b11111111 => {
                    // c & d are exponent MSBs
                    let c = if (byte | 0b11101111) == max { 1 } else { 0 };
                    let d = if (byte | 0b11110111) == max { 1 } else { 0 };
                    // e is the last of the coefficient MSD bits
                    let e = if (byte | 0b11111011) == max { 1 } else { 0 };
                    let mut exp = bitvec![c, d];
                    total_exp.append(&mut exp);
                    let coef = bitvec![1, 0, 0, e];
                    CombinationField::Finite(Exponent(exp), Coefficient(coef))
                }
                _ => {
                    // a & b are exponent MSBs
                    let a = if (byte | 0b10111111) == max { 1 } else { 0 };
                    let b = if (byte | 0b11011111) == max { 1 } else { 0 };
                    // c, d, and e make up the last three coefficient MSD bits
                    let c = if (byte | 0b11101111) == max { 1 } else { 0 };
                    let d = if (byte | 0b11110111) == max { 1 } else { 0 };
                    let e = if (byte | 0b11111011) == max { 1 } else { 0 };
                    let mut exp = bitvec![a, b];
                    total_exp.append(&mut exp);
                    let coef = bitvec![0, c, d, e];
                    CombinationField::Finite(Exponent(exp), Coefficient(coef))
                }
            },
        };

        let next_byte = rdr.read_u8().unwrap();
        let bit_vec = unsafe { BitVec::from_raw_parts(next_byte, 8) };

        let dec128 = match combination_field {
            CombinationField::Finite(exp, coef) => Decimal128 {
                sign,
                exponent: Some(exp),
                coefficient: Some(coef),
            },
            _ => Decimal128 {
                sign,
                exponent: None,
                coefficient: None,
            },
        };
        Ok(dec128)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
