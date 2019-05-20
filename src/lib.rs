//! Decimal 128 bits are broken down like so:
//! [1bits]  [   14bits   ]  [   113 bits   ]
//!  sign       exponent        significand
//!              field  
use bitvec::{bitvec, BigEndian, BitVec};
use byteorder::*;
use failure::ensure;
use hex::*;
use std::io::Cursor;

#[derive(Debug, Clone)]
pub struct Exponent {
    vec: BitVec<BigEndian>,
}
#[derive(Debug, Clone)]
pub struct Significand {
    vec: BitVec<BigEndian>,
}

pub struct Decimal128 {
    pub sign: bool,
    pub exponent: Exponent,
    pub significand: Significand,
    nan: bool,
    inf: bool,
}

pub enum NumberType {
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
    /// let vec: [u8; 16] = [9, 16, 3, 6, 7, 86, 76, 81, 89, 0, 3, 45, 12, 71, 52, 39];
    /// let dec128 = Decimal128::from_raw_buf(vec).unwrap();
    /// ```
    pub fn from_raw_buf(buffer: [u8; 16]) -> Result<Self, failure::Error> {
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
            0b1111_1111 => NumberType::NaN,
            // if the last of the five bits is a 0, we are looking at Infinity
            0b1111_1011 => NumberType::Infinity,
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
                    NumberType::Finite
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
                    NumberType::Finite
                }
            },
        };

        // the rest of the bytes of the vec we are passed in.
        for bytes in 3..buffer.len() {
            let mut bv: BitVec = (&[buffer[bytes]] as &[u8]).into();
            total_sig.append(&mut bv);
        }

        let dec128 = match combination_field {
            NumberType::Finite => Decimal128 {
                sign,
                exponent: total_exp,
                significand: total_sig,
                nan: false,
                inf: false,
            },
            NumberType::NaN => Decimal128 {
                sign,
                exponent: total_exp,
                significand: total_sig,
                nan: true,
                inf: false,
            },
            NumberType::Infinity => Decimal128 {
                sign,
                exponent: total_exp,
                significand: total_sig,
                nan: false,
                inf: true,
            },
        };
        Ok(dec128)
    }

    /// Converts Decimal128 to string. Uses information in
    /// [speleotrove](http://speleotrove.com/decimal/daconvs.html) decimal
    /// documentation.
    pub fn to_string(&self) -> String {
        // just return NaN if we are dealing with NaN. This does not come with a
        // sign.
        if self.nan {
            return String::from("NaN");
        };

        // Everything else can have a sign. We can create a string from Infinity
        // or a Finite number.
        let str = if self.inf {
            "Infinity".to_string()
        } else {
            self.create_string()
        };

        // add a sign if this is a negative number
        return if !self.sign { str } else { format!("-{}", str) };
    }

    fn create_string(&self) -> String {
        println!("significand {:?}", self.significand.to_num());
        println!("exponent {:?}", self.exponent.to_num());
        println!("adjusted exponent {:?}", self.exponent.to_adjusted());
        if self.use_scientific_notation() {
            let exp_sign = if self.exponent.to_adjusted() < 0 {
                ""
            } else {
                "+"
            };

            if self.significand.as_digit_vec().len() > 1 {
                let mut first_significand = self.significand.as_digit_vec().clone();
                let remainder = first_significand.split_off(1);
                let remainder_significand = remainder
                    .into_iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<String>>()
                    .join("");
                return format!(
                    "{first_significand}.{remainder_significand}E{exp_sign}{scientific_exponent}",
                    first_significand = first_significand[0],
                    remainder_significand = remainder_significand,
                    exp_sign = exp_sign,
                    scientific_exponent = self.scientific_exponent()
                );
            } else {
                return format!(
                    "{significand}E{exp_sign}{scientific_exponent}",
                    significand = self.significand.to_num(),
                    exp_sign = exp_sign,
                    scientific_exponent = self.scientific_exponent()
                );
            }
        } else if self.exponent.to_adjusted() < 0 {
            if self.significand.count_digits() > self.exponent.to_adjusted().abs() {
                let decimal_point_index =
                    self.significand.count_digits() - self.exponent.to_adjusted().abs();
                let mut first_significand = self.significand.as_digit_vec().clone();
                let remainder = first_significand.split_off(decimal_point_index as usize - 1);
                let remainder_significand = remainder
                    .into_iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<String>>()
                    .join("");
                return format!(
                    "{first_significand}.{remainder_significand}",
                    first_significand = first_significand[0],
                    remainder_significand = remainder_significand
                );
            } else {
                let left_zero_pad_count =
                    (self.exponent.to_adjusted() + self.significand.count_digits()).abs();
                let zero_pad = std::iter::repeat("0")
                    .take(left_zero_pad_count as usize)
                    .collect::<String>();
                return format!(
                    "0.{zero_pad}{significand}",
                    zero_pad = zero_pad,
                    significand = self.significand.to_num()
                );
            }
        }
        format!("{}", self.significand.to_num())
    }

    fn use_scientific_notation(&self) -> bool {
        (self.exponent.to_adjusted() as i16) > 0 || (self.scientific_exponent() as i16) < -6
    }

    // TODO: check if we can just return a number here
    // TODO: match up number types with significand and exponenet
    fn scientific_exponent(&self) -> i16 {
        // first variable is number of digits in a significand
        (self.significand.count_digits() - 1) + self.exponent.to_adjusted()
    }
}

/// Exponent is a 14-bit portion of decimal128 that follows the sign bit. Here we
/// are storing it as a 16-bit BitVec that can be later converted to a u16.
impl Exponent {
    pub fn new() -> Self {
        Exponent {
            vec: bitvec![BigEndian, u8; 0; 2],
        }
    }

    pub fn append(&mut self, vec: &mut BitVec) {
        self.vec.append(vec)
    }

    pub fn to_num(&self) -> u16 {
        let mut reader = Cursor::new(&self.vec);
        reader.read_u16::<byteorder::BigEndian>().unwrap()
    }

    // compare current exponent value with exponent bias (largest possible
    // exponent value)
    // TODO: check if 6176 (exponent bias) can be stored as u16
    pub fn to_adjusted(&self) -> i16 {
        self.to_num() as i16 - 6176 as i16
    }
}

/// Significand is a padded 111- or 113-bit coefficient. We are storing it as a
/// 128-bit BitVec with the padded difference. This can be converted to a u128.
impl Significand {
    pub fn new() -> Self {
        Significand {
            vec: bitvec![BigEndian, u8; 0; 14],
        }
    }

    pub fn append(&mut self, vec: &mut BitVec) {
        self.vec.append(vec)
    }

    pub fn to_num(&self) -> u128 {
        let mut reader = Cursor::new(&self.vec);
        reader.read_u128::<byteorder::BigEndian>().unwrap()
    }

    // count the number of digits in the significand. This method first converts
    // significand BitVec into a u128 number, then converts it to string to
    // count characters and collects them in a vec to look at the vec's length.
    //
    // We return a u16 number of digits, as it's easier to compare to the
    // exponent since that's also stored as a u16.
    fn count_digits(&self) -> i16 {
        self.as_digit_vec().len() as i16
    }

    fn as_digit_vec(&self) -> Vec<u32> {
        let digits: Vec<u32> = self
            .to_num()
            .to_string()
            .chars()
            .map(|c| c.to_digit(10).unwrap())
            .collect();
        return digits;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_returns_negative_infinity() {
        let vec: [u8; 16] = [
            0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        let dec128 = Decimal128::from_raw_buf(vec).unwrap();
        let string = dec128.to_string();
        assert_eq!("-Infinity".to_string(), string);
    }
    #[test]
    fn it_returns_positive_infinity() {
        let vec: [u8; 16] = [
            0x78, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        let dec128 = Decimal128::from_raw_buf(vec).unwrap();
        let string = dec128.to_string();
        assert_eq!("Infinity".to_string(), string);
    }

    #[test]
    fn it_returns_nan() {
        let vec: [u8; 16] = [
            0x7c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        let dec128 = Decimal128::from_raw_buf(vec).unwrap();
        let string = dec128.to_string();
        assert_eq!("NaN".to_string(), string);
    }

    #[test]
    fn it_returns_0_001234() {
        let mut vec: [u8; 16] = [
            0x30, 0x34, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x04, 0xd2,
        ];
        let dec128 = Decimal128::from_raw_buf(vec).unwrap();
        let decimal = dec128.to_string();
        assert_eq!("0.001234".to_string(), decimal);
    }

    #[test]
    fn it_returns_123456789012() {
        let vec: [u8; 16] = [
            0x30, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1c, 0xbe, 0x99,
            0x1a, 0x14,
        ];
        let dec128 = Decimal128::from_raw_buf(vec).unwrap();
        let decimal = dec128.to_string();
        assert_eq!("123456789012".to_string(), decimal)
    }

    #[test]
    fn it_returns_0_00123400000() {
        let vec: [u8; 16] = [
            0x30, 0x2a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x5a,
            0xef, 0x40,
        ];
        let dec128 = Decimal128::from_raw_buf(vec).unwrap();
        let decimal = dec128.to_string();
        assert_eq!("0.00123400000".to_string(), decimal)
    }

    #[test]
    fn it_returns_0_1234567890123456789012345678901234() {
        let vec: [u8; 16] = [
            0x2f, 0xfc, 0x3c, 0xde, 0x6f, 0xff, 0x97, 0x32, 0xde, 0x82, 0x5c, 0xd0, 0x7e, 0x96,
            0xaf, 0xf2,
        ];
        let dec128 = Decimal128::from_raw_buf(vec).unwrap();
        let decimal = dec128.to_string();
        assert_eq!("0.1234567890123456789012345678901234".to_string(), decimal)
    }

    #[test]
    fn it_returns_1_000000000000000000000000000000000E_6144() {
        let vec: [u8; 16] = [
            0x5f, 0xfe, 0x31, 0x4d, 0xc6, 0x44, 0x8d, 0x93, 0x38, 0xc1, 0x5b, 0x0a, 0x00, 0x00,
            0x00, 0x00,
        ];
        let dec128 = Decimal128::from_raw_buf(vec).unwrap();
        let decimal = dec128.to_string();
        assert_eq!(
            "1.000000000000000000000000000000000E+6144".to_string(),
            decimal
        )
    }

    #[test]
    fn it_returns_1E_6176() {
        let vec: [u8; 16] = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01,
        ];
        let dec128 = Decimal128::from_raw_buf(vec).unwrap();
        let decimal = dec128.to_string();
        assert_eq!("1E-6176".to_string(), decimal)
    }
}
