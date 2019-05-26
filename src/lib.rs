//! Decimal 128 bits are broken down like so:
//! [1bits]  [   14bits   ]  [   113 bits   ]
//!  sign       exponent        significand
//!              field  
use bitvec::{bitvec, BigEndian, BitVec};
use byteorder::*;
use std::cmp::Ordering;
use std::fmt;
use std::io::Cursor;

#[derive(Clone, PartialEq, PartialOrd)]
pub struct Exponent {
    vec: BitVec<BigEndian>,
}
#[derive(Clone, PartialEq, PartialOrd)]
pub struct Significand {
    vec: BitVec<BigEndian>,
}

#[derive(Clone)]
pub struct Decimal128 {
    pub sign: bool,
    pub exponent: Exponent,
    pub significand: Significand,
    pub bytes: [u8; 16],
    nan: bool,
    inf: bool,
}

pub enum NumberType {
    NaN,
    Infinity,
    Finite,
}

impl Decimal128 {
    /// Create a Decimal128 from a [u8; 16].
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
    pub fn from_raw_buf(buffer: [u8; 16]) -> Self {
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
                bytes: buffer,
                nan: false,
                inf: false,
            },
            NumberType::NaN => Decimal128 {
                sign,
                exponent: total_exp,
                significand: total_sig,
                bytes: buffer,
                nan: true,
                inf: false,
            },
            NumberType::Infinity => Decimal128 {
                sign,
                exponent: total_exp,
                significand: total_sig,
                bytes: buffer,
                nan: false,
                inf: true,
            },
        };
        dec128
    }

    pub fn is_nan(&self) -> bool {
        if self.nan {
            return true;
        } else {
            return false;
        }
    }

    pub fn is_negative(&self) -> bool {
        if self.sign {
            return true;
        } else {
            return false;
        }
    }

    pub fn is_positive(&self) -> bool {
        return !self.is_negative();
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
        if self.use_scientific_notation() {
            let exp_sign = if self.exponent.to_adjusted() < 0 {
                ""
            } else {
                "+"
            };

            if self.significand.as_digit_vec().len() > 1 {
                let mut first_significand = self.significand.as_digit_vec().clone();
                // we already used the first digit, so only stringify the
                // remainder of the significand
                let remainder_significand = stringify_vec(first_significand.split_off(1));
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
                let dec_point = self.get_decimal_point_index() as usize;
                let mut significand_vec = self.significand.as_digit_vec().clone();
                let remainder_significand = stringify_vec(significand_vec.split_off(dec_point - 1));
                return format!(
                    "{first_significand}.{remainder_significand}",
                    first_significand = significand_vec[0],
                    remainder_significand = remainder_significand
                );
            } else {
                let zero_pad = self.get_zero_padding();
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

    fn scientific_exponent(&self) -> i16 {
        // first variable is number of digits in a significand
        (self.significand.count_digits() - 1) + self.exponent.to_adjusted()
    }

    // for larger numbers we want to know where to put the decimal point.
    fn get_decimal_point_index(&self) -> i16 {
        self.significand.count_digits() - self.exponent.to_adjusted().abs()
    }

    // for very small decimals, we need to know how many zeroes to pad it with.
    fn get_zero_padding(&self) -> String {
        let left_zero_pad_count =
            (self.exponent.to_adjusted() + self.significand.count_digits()).abs();
        std::iter::repeat("0")
            .take(left_zero_pad_count as usize)
            .collect::<String>()
    }

    /// create a compare functiont that returns a decimal 128 that's either:
    /// * -1 = less than
    /// * 0 = equal
    /// * 1 = greater than
    /// When comparing and orderign Decimal128, we should end up with:
    /// (-) NaN | -Infinity | x < 0 | -0 | +0 | x > 0 | +Infinity | (+) NaN
    ///
    /// Even though NaN can't be negative or positive, when reading the sign bit,
    /// (-) NaN < (+) NaN
    //
    // TODO: once we have a method to create Decimal128 from another number type
    // (u32/i32/u128/i128), change this return type to be a Decimal128 as well.
    pub fn compare(&self, other: &Decimal128) -> isize {
        let self_exp = self.exponent.to_adjusted();
        let other_exp = other.exponent.to_adjusted();
        let self_signif = self.significand.to_num();
        let other_signif = other.significand.to_num();

        // NaN and Infinity will be ordered via the sign Check
        if self.sign > other.sign {
            -1
        } else if self.sign < other.sign {
            1
        } else {
            // since 1x10^3 is the same number as 10x10^2, we want to try to
            // even out the exponents before comparing significands.
            let exp_dif = (self_exp - other_exp).abs();
            // however, if the difference is greeater than 66, they are
            // definitely diffferent numbers. so we only try to mingle with
            // exponents if the difference is less than 66.
            if exp_dif <= 66 {
                if self_exp < other_exp {
                    Decimal128::increase_exponent(self_signif, self_exp, other_exp);
                    Decimal128::decrease_exponent(other_signif, other_exp, self_exp);
                } else if self_exp > other_exp {
                    Decimal128::decrease_exponent(self_signif, self_exp, other_exp);
                    Decimal128::increase_exponent(other_signif, other_exp, self_exp);
                }
            }
            if self_exp == other_exp {
                if self_signif > other_signif {
                    1
                } else if self_signif < other_signif {
                    -1
                } else {
                    0
                }
            } else {
                if self_exp > other_exp {
                    1
                } else if self_exp < other_exp {
                    -1
                } else {
                    0
                }
            }
        }
    }

    // This is part of the effort to compare two different Decimal128 numbers.
    fn increase_exponent(mut significand: u128, mut exponent: i16, goal: i16) {
        if significand == 0 as u128 {
            exponent = goal
        }

        while exponent < goal {
            let significand_divided_by_10 = significand / 10;
            if significand % 10 != 0 {
                break;
            }
            exponent += 1;
            significand = significand_divided_by_10
        }
    }

    // This is part of the effort to compare two different Decimal128 numbers.
    fn decrease_exponent(mut significand: u128, mut exponent: i16, goal: i16) {
        if significand == 0 as u128 {
            exponent = goal
        }

        while exponent > goal {
            let significand_times_10 = significand * 10;
            if significand_times_10 - Significand::max_value() > 0 {
                break;
            }
            exponent -= 1;
            significand = significand_times_10
        }
    }
}

impl fmt::Display for Decimal128 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.to_string())
    }
}

// this should be the same as Display trait
impl fmt::Debug for Decimal128 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, fmt)
    }
}

impl PartialOrd<Decimal128> for Decimal128 {
    fn partial_cmp(&self, other: &Decimal128) -> Option<Ordering> {
        match self.compare(other) {
            v if v == 0 => Some(Ordering::Equal),
            v if v > 0 => Some(Ordering::Greater),
            v if v < 0 => Some(Ordering::Less),
            _ => None,
        }
    }
}

impl PartialEq<Decimal128> for Decimal128 {
    fn eq(&self, other: &Decimal128) -> bool {
        self.compare(other) == 0
    }
}

/// Format Decimal128 as an engineering string
/// TODO: this currently only uses the default to_string method for Decimal128
/// and needs to actually do the engineering string formatting.
impl fmt::LowerExp for Decimal128 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, fmt)
    }
}
/// Formats Decimal128 to hexadecimal binary representation.
impl fmt::LowerHex for Decimal128 {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        for b in self.bytes.iter().rev() {
            write!(fmt, "{:02x}", b)?;
        }
        Ok(())
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

    pub fn max_value() -> u128 {
        u128::from_str_radix("9999999999999999999999999999999999", 10).unwrap()
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

fn stringify_vec(vec: Vec<u32>) -> String {
    vec.into_iter()
        .map(|d| d.to_string())
        .collect::<Vec<String>>()
        .join("")
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
        let dec128 = Decimal128::from_raw_buf(vec);
        let string = dec128.to_string();
        assert_eq!("-Infinity".to_string(), string);
    }
    #[test]
    fn it_returns_positive_infinity() {
        let vec: [u8; 16] = [
            0x78, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        let dec128 = Decimal128::from_raw_buf(vec);
        let string = dec128.to_string();
        assert_eq!("Infinity".to_string(), string);
    }

    #[test]
    fn it_returns_nan() {
        let vec: [u8; 16] = [
            0x7c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        let dec128 = Decimal128::from_raw_buf(vec);
        let string = dec128.to_string();
        assert_eq!("NaN".to_string(), string);
    }

    #[test]
    fn it_returns_0_001234() {
        let vec: [u8; 16] = [
            0x30, 0x34, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x04, 0xd2,
        ];
        let dec128 = Decimal128::from_raw_buf(vec);
        let decimal = dec128.to_string();
        assert_eq!("0.001234".to_string(), decimal);
    }

    #[test]
    fn it_returns_123456789012() {
        let vec: [u8; 16] = [
            0x30, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1c, 0xbe, 0x99,
            0x1a, 0x14,
        ];
        let dec128 = Decimal128::from_raw_buf(vec);
        let decimal = dec128.to_string();
        assert_eq!("123456789012".to_string(), decimal)
    }

    #[test]
    fn it_returns_0_00123400000() {
        let vec: [u8; 16] = [
            0x30, 0x2a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x5a,
            0xef, 0x40,
        ];
        let dec128 = Decimal128::from_raw_buf(vec);
        let decimal = dec128.to_string();
        assert_eq!("0.00123400000".to_string(), decimal)
    }

    #[test]
    fn it_returns_0_1234567890123456789012345678901234() {
        let vec: [u8; 16] = [
            0x2f, 0xfc, 0x3c, 0xde, 0x6f, 0xff, 0x97, 0x32, 0xde, 0x82, 0x5c, 0xd0, 0x7e, 0x96,
            0xaf, 0xf2,
        ];
        let dec128 = Decimal128::from_raw_buf(vec);
        let decimal = dec128.to_string();
        assert_eq!("0.1234567890123456789012345678901234".to_string(), decimal)
    }

    #[test]
    fn it_returns_1_000000000000000000000000000000000_e_6144() {
        let vec: [u8; 16] = [
            0x5f, 0xfe, 0x31, 0x4d, 0xc6, 0x44, 0x8d, 0x93, 0x38, 0xc1, 0x5b, 0x0a, 0x00, 0x00,
            0x00, 0x00,
        ];
        let dec128 = Decimal128::from_raw_buf(vec);
        let decimal = dec128.to_string();
        assert_eq!(
            "1.000000000000000000000000000000000E+6144".to_string(),
            decimal
        )
    }

    #[test]
    fn it_returns_1_e_6176() {
        let vec: [u8; 16] = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01,
        ];
        let dec128 = Decimal128::from_raw_buf(vec);
        let decimal = dec128.to_string();
        assert_eq!("1E-6176".to_string(), decimal)
    }
}
