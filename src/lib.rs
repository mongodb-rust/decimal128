//! Decimal 128 bits are broken down like so:
//! [1bits]  [   14bits   ]  [   113 bits   ]
//!  sign       exponent        significand
//!              field  

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

#[derive(Clone)]
pub struct Decimal128 {
    sign: bool,
    exp: u16,
    sig: u128,
    bytes: [u8; 16],
    nan: bool,
    inf: bool,
}

enum NumberType {
    NaN,
    Infinity,
    Finite,
}

impl From<i32> for Decimal128 {
    fn from(_v: i32) -> Self {
        unimplemented!("Creating Decimal128 from i32 is not yet implemented.")
    }
}

impl From<u32> for Decimal128 {
    fn from(_v: u32) -> Self {
        unimplemented!("Creating Decimal128 from u32 is not yet implemented.")
    }
}

impl FromStr for Decimal128 {
    type Err = ();
    fn from_str(_s: &str) -> Result<Self, ()> {
        unimplemented!("Creating Decimal128 from string is not yet implemented.")
    }
}

impl Into<i32> for Decimal128 {
    fn into(self) -> i32 {
        unimplemented!("Creating i32 from Decimal128 is not yet implemented.")
    }
}

impl Into<u32> for Decimal128 {
    fn into(self) -> u32 {
        unimplemented!("Creating u32 from Decimal128 is not yet implemented.")
    }
}

impl Decimal128 {
    //     pub fn zero() -> Self {
    //         Decimal128 {
    //             sign: false,
    //             exponent: Exponent::new(),
    //             significand: Significand::new(),
    //             bytes: [0u8; 16],
    //             nan: false,
    //             inf: false,
    //         }
    //     }
    //
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
    /// let dec128 = Decimal128::from_raw_bytes(vec);
    /// ```
    pub fn from_raw_bytes(buffer: [u8; 16]) -> Self {
        let mut num = Decimal128 {
            sign: false,
            // decimal 128's exponent is 14bits long. We will construct a u16 to
            // begin with. The first two bits will be 0, and the rest will be
            // swapped out as bits come in.
            exp: 0,
            // Significand can be 113 *or* 111 bit long. It will start off as a
            // u128. The first 14 bits will be 0s and the rest will be swapped out
            // as the rest of the bits come in.
            sig: 0,
            bytes: [0u8; 16],
            nan: false,
            inf: false,
        };

        // first byte
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
            // TODO: clarify comment
            // if the last of the five bits is a 0, we are looking at Infinity
            0b1111_1011 => NumberType::Infinity,
            // match for finite cases
            _ => match byte | 0b1001_1111 {
                0b1111_1111 => {
                    // since the first two bits after the sign are 11, we ignore
                    // them and gather the remainder of the first byte.
                    //
                    // 16 bits total:
                    // - 2 zeroes
                    // - 5 exponent bits
                    // - 8 more exponent bits
                    // - 1 more exponent bit
                    if (byte | 0b1110_1111) == max {
                        num.exp |= 1 << 13;
                    }
                    if (byte | 0b1111_0111) == max {
                        num.exp |= 1 << 12;
                    }
                    if (byte | 0b1111_1011) == max {
                        num.exp |= 1 << 11;
                    }
                    if (byte | 0b1111_1101) == max {
                        num.exp |= 1 << 10;
                    }
                    if (byte | 0b1111_1110) == max {
                        num.exp |= 1 << 9;
                    }

                    // fill the u16 exponent with the entire second byte from bit 7 to bit 15.
                    num.exp |= (buffer[1] as u16) << 1;

                    // out of the third byte the first bit is part of the
                    // exponent, and the last 7 bits are part of the significand
                    let byte_3 = buffer[2];
                    if (byte_3 | 0b0111_1111) == max {
                        num.exp |= 1;
                    }

                    // significand is 128 bits
                    // when are shifting our math is based on total of 127.
                    // the first 14 bits of 128 bits need to be 0s
                    // 127-14 = 113

                    // Start a new vec for 111bit significand. This version of
                    // the significand is offset by two bits, so we pad it with
                    // `100`
                    num.sig |= 1 << 113;
                    num.sig |= 0 << 112;
                    num.sig |= 0 << 111;

                    if (byte | 0b1011_1111) == max {
                        num.sig |= 1 << 110;
                    }
                    if (byte | 0b1101_1111) == max {
                        num.sig |= 1 << 109;
                    }
                    if (byte | 0b1110_1111) == max {
                        num.sig |= 1 << 108;
                    }
                    if (byte | 0b1111_0111) == max {
                        num.sig |= 1 << 107;
                    }
                    if (byte | 0b1111_1011) == max {
                        num.sig |= 1 << 106;
                    }
                    if (byte | 0b1111_1101) == max {
                        num.sig |= 1 << 105;
                    }
                    if (byte | 0b1111_1110) == max {
                        num.sig |= 1 << 104;
                    }
                    NumberType::Finite
                }
                _ => {
                    // if the first two bits after the sign are `00`, `01`,
                    // `10`, we add the remainder of the first byte to exponent

                    // we are filling up a u16 (16bits), but exponent is only 14,
                    // so we need to leave the first two bits as 00
                    // 15 - 2 = 13
                    if (byte | 0b1011_1111) == max {
                        num.exp |= 1 << 13;
                    }
                    if (byte | 0b1101_1111) == max {
                        num.exp |= 1 << 12;
                    }
                    if (byte | 0b1110_1111) == max {
                        num.exp |= 1 << 11;
                    }
                    if (byte | 0b1111_0111) == max {
                        num.exp |= 1 << 10;
                    }
                    if (byte | 0b1111_1011) == max {
                        num.exp |= 1 << 9;
                    }
                    if (byte | 0b1111_1101) == max {
                        num.exp |= 1 << 8;
                    }
                    if (byte | 0b1111_1110) == max {
                        num.exp |= 1 << 7;
                    }
                    // out of the second byte the first 7 bits are part of the
                    // exponent, and the last bit is part of the significand
                    let byte_2 = buffer[1];
                    if (byte_2 | 0b0111_1111) == max {
                        num.exp |= 1 << 6;
                    }
                    if (byte_2 | 0b1011_1111) == max {
                        num.exp |= 1 << 5;
                    }
                    if (byte_2 | 0b1101_1111) == max {
                        num.exp |= 1 << 4;
                    }
                    if (byte_2 | 0b1110_1111) == max {
                        num.exp |= 1 << 3;
                    }
                    if (byte_2 | 0b1111_0111) == max {
                        num.exp |= 1 << 2;
                    }
                    if (byte_2 | 0b1111_1011) == max {
                        num.exp |= 1 << 1;
                    }
                    if (byte_2 | 0b1111_1101) == max {
                        num.exp |= 1;
                    }
                    // Start a new vec for 113bit significand. Since this
                    // version of significand is not offset, we pad it with only
                    // `0`
                    num.sig |= 0 << 113;
                    if (byte_2 | 0b1111_1110) == max {
                        num.sig |= 112;
                    }
                    // add the whole third byte to the signficand in this case
                    num.sig |= (buffer[2] as u128) << 111;
                    NumberType::Finite
                }
            },
        };

        // the rest of the bytes of the vec are part of the
        // significand. We can bit shift them all in.
        // 111 - 8 = 103
        for bytes in 3..buffer.len() {
            num.sig |= (buffer[bytes] as u128) << 103;
        }

        let dec128 = match combination_field {
            NumberType::Finite => Decimal128 {
                sign,
                exp: num.exp,
                sig: num.sig,
                bytes: buffer,
                nan: false,
                inf: false,
            },
            NumberType::NaN => Decimal128 {
                sign,
                exp: num.exp,
                sig: num.sig,
                bytes: buffer,
                nan: true,
                inf: false,
            },
            NumberType::Infinity => Decimal128 {
                sign,
                exp: num.exp,
                sig: num.sig,
                bytes: buffer,
                nan: false,
                inf: true,
            },
        };
        dec128
    }

    pub fn is_nan(&self) -> bool {
        self.nan
    }

    pub fn is_negative(&self) -> bool {
        self.sign
    }

    pub fn is_positive(&self) -> bool {
        !self.is_negative()
    }

    pub fn is_zero(&self) -> bool {
        !self.nan && self.exp == 0 && self.sig == 0
    }

    /// Converts Decimal128 to string. Uses information in
    /// [speleotrove](http://speleotrove.com/decimal/daconvs.html) decimal
    /// documentation.
    pub fn to_string(&self) -> String {
        // Just return the string 'NaN' if we are dealing with NaN.
        // This does *not* come with a 'sign'.
        if self.nan {
            return String::from("NaN");
        };

        // Everything else can have a sign.

        // We can create a string from 'Infinity' or a Finite number.
        let str = if self.inf {
            "Infinity".to_string()
        } else {
            self.create_string()
        };

        // Specifically add a '-' sign to our string if this is a negative number.
        return if !self.sign { str } else { format!("-{}", str) };
    }

    /// Returns raw bytes.
    pub fn to_raw_bytes(&self) -> [u8; 16] {
        self.bytes
    }

    fn create_string(&self) -> String {
        if self.use_scientific_notation() {
            let exp_sign = if self.adjust_exponent() < 0 {
                ""
            } else {
                "+"
            };

            if self.sig_as_digit_vec().len() > 1 {
                let mut first_significand = self.sig_as_digit_vec().clone();
                // We already used the first digit, so only stringify the
                // remainder of the significand.
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
                    significand = self.sig,
                    exp_sign = exp_sign,
                    scientific_exponent = self.scientific_exponent()
                );
            }
        } else if self.adjust_exponent() < 0 {
            if self.count_sig_digits() > self.adjust_exponent().abs() {
                let dec_point = self.get_decimal_point_index() as usize;
                let mut significand_vec = self.sig_as_digit_vec().clone();
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
                    significand = self.sig
                );
            }
        }
        format!("{}", self.sig)
    }

    fn use_scientific_notation(&self) -> bool {
        self.adjust_exponent() > 0 || (self.scientific_exponent() as i16) < -6
    }

    fn scientific_exponent(&self) -> i16 {
        // first variable is number of digits in a significand
        (self.count_sig_digits() - 1) + self.adjust_exponent()
    }

    // Compare current exponent value with exponent bias, 6176, which is also the largest
    // possible exponent value.
    //
    // We convert exp and exponent bias to an i16 as this number _can_ be signed.
    fn adjust_exponent(&self) -> i16 {
       self.exp as i16 - 6176 as i16
    }

    // Convert significand into a vec of numbers.
    // For example, if the number is 128765, the vec will look like [1, 2, 8, 7, 6, 5]
    fn sig_as_digit_vec(&self) -> Vec<u32> {
        let digits: Vec<u32> = self.sig
            .to_string()
            .chars()
            .map(|c| c.to_digit(10).unwrap())
            .collect();
        return digits;
    }

    // Count the number of digits in the significand.
    //
    // This method first converts significand into a digit vec.
    //
    // We then return a u16 number of digits, as it's easier to compare to the
    // exponent since that's also stored as a u16.
    fn count_sig_digits(&self) -> i16 {
        self.sig_as_digit_vec().len() as i16
    }

    // Determines where to put the decimal point for larger numbers.
    fn get_decimal_point_index(&self) -> i16 {
        self.count_sig_digits() - self.adjust_exponent().abs()
    }

    // Determines how many zeroes to pad smaller numbers with.
    fn get_zero_padding(&self) -> String {
        let left_zero_pad_count: i16 =
            (self.adjust_exponent() + self.count_sig_digits()).abs();
        std::iter::repeat("0")
            .take(left_zero_pad_count as usize)
            .collect::<String>()
    }

    /// Creates a compare function that returns a decimal 128 that's either:
    /// * -1 = less than
    /// * 0 = equal
    /// * 1 = greater than
    ///
    /// When comparing and ordering Decimal128, we should end up with:
    /// (-) NaN | -Infinity | x < 0 | -0 | +0 | x > 0 | +Infinity | (+) NaN
    ///
    /// Even though NaN can't be negative or positive, when reading the sign bit,
    /// (-) NaN < (+) NaN
    //
    // TODO: once we have a method to create Decimal128 from another number type
    // (u32/i32/u128/i128), change this return type to be a Decimal128 as well.
    pub fn compare(&self, other: &Decimal128) -> isize {
        let self_exp = self.adjust_exponent();
        let other_exp = other.adjust_exponent();
        let self_sig = self.sig;
        let other_sig = other.sig;

        // NaN and Infinity will be ordered via the sign Check
        if self.sign > other.sign {
            -1
        } else if self.sign < other.sign {
            1
        } else {
            // since 1x10^3 is the same number as 10x10^2, we want to try to
            // even out the exponents before comparing significands.
            let exp_dif = (self_exp - other_exp).abs();
            // however, if the difference is greater than 66, they are
            // definitely diffferent numbers. so we only try to mingle with
            // exponents if the difference is less than 66.
            if exp_dif <= 66 {
                if self_exp < other_exp {
                    Decimal128::increase_exponent(self_sig, self_exp, other_exp);
                    Decimal128::decrease_exponent(other_sig, other_exp, self_exp);
                } else if self_exp > other_exp {
                    Decimal128::decrease_exponent(self_sig, self_exp, other_exp);
                    Decimal128::increase_exponent(other_sig, other_exp, self_exp);
                }
            }
            if self_exp == other_exp {
                if self_sig > other_sig {
                    1
                } else if self_sig < other_sig {
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
        let max_sig = u128::from_str_radix("9999999999999999999999999999999999", 10).unwrap();
        if significand == 0 as u128 {
            exponent = goal
        }

        while exponent > goal {
            let significand_times_10 = significand * 10;
            if (significand_times_10 - max_sig) > 0 {
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


fn stringify_vec(vec: Vec<u32>) -> String {
    vec.into_iter()
        .map(|d| d.to_string())
        .collect::<Vec<String>>()
        .join("")
}
