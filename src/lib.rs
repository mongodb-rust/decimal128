//! Decimal 128 bits are broken down like so:
//! [1bits]  [   14bits   ]  [   113 bits   ]
//!  sign       exponent        significand
//!              field  
use bitvec::*;
use failure::ensure;

#[derive(Debug, Clone)]
pub struct Exponent(BitVec<LittleEndian>);
#[derive(Debug, Clone)]
pub struct Significand(BitVec<LittleEndian>);

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
        let mut total_exp = bitvec![LittleEndian, u8; 0; 2];
        // Significnad can be 113 *or* 111 bit long. Regardless of the size we
        // will pad it with 14 0s. We will be eventually constructing a u128
        // from this eventually.
        let mut total_sig = bitvec![LittleEndian, u8; 0; 14];

        let byte = buffer[0];
        let max = 0b11111111;
        // first bit is sign: negative or positive integer
        let is_negative_bitmask = 0b01111111;
        let sign = (byte | is_negative_bitmask) == max;

        // the next 5 bits of the first byte are combination field; these include:
        // first 5 bits       Type	    Exponent MSBs	Significand MSD
        // ---------------------------------------------------------------------------
        //     a b c d e	  Finite       14bits           113bits
        //     1 1 c d e	  Finite    2 bits to right     111bits
        //     1 1 1 1 0	  Infinity	    - -	            - - - -
        //     1 1 1 1 1	  NaN           - -             - - - -
        // the easiest bitmask to do is for NaN, i.e. five 1s
        let res = byte | 0b10000011;
        let combination_field = match res {
            // if everything is 1s, we are looking at NaN
            0b11111111 => CombinationField::NaN,
            // if the last of the five bits is a 0, we are looking at Infinity
            0b11111011 => CombinationField::Infinity,
            // match for finite cases
            _ => match byte | 0b1001111 {
                0b11111111 => {
                    // since the first two bits after the sign are 11, we ignore
                    // them and gather the remainder of the first byte.
                    let c = if (byte | 0b11101111) == max { 1 } else { 0 };
                    let d = if (byte | 0b11110111) == max { 1 } else { 0 };
                    let e = if (byte | 0b11111011) == max { 1 } else { 0 };
                    let f = if (byte | 0b11111101) == max { 1 } else { 0 };
                    let g = if (byte | 0b11111110) == max { 1 } else { 0 };
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
                    let a = if (byte_2 | 0b01111111) == max { 1 } else { 0 };
                    let mut exp_cont = bitvec![a];
                    total_exp.append(&mut exp_cont);
                    let b = if (byte_3 | 0b10111111) == max { 1 } else { 0 };
                    let c = if (byte_3 | 0b11011111) == max { 1 } else { 0 };
                    let d = if (byte_3 | 0b11101111) == max { 1 } else { 0 };
                    let e = if (byte_3 | 0b11110111) == max { 1 } else { 0 };
                    let f = if (byte_3 | 0b11111011) == max { 1 } else { 0 };
                    let g = if (byte_3 | 0b11111101) == max { 1 } else { 0 };
                    let h = if (byte_3 | 0b11111110) == max { 1 } else { 0 };
                    // Start a new vec for 111bit significand. This version of
                    // the significand is offset by two bits, so we pad it with
                    // `100`
                    let mut sig = bitvec![1, 0, 0, b, c, d, e, f, g, h];
                    total_sig.append(&mut sig);
                    CombinationField::Finite
                }
                _ => {
                    // if the first two bits after the sign are `00`, `01`,
                    // `10`, we add the remainder of the first byte to exponent
                    let a = if (byte | 0b10111111) == max { 1 } else { 0 };
                    let b = if (byte | 0b11011111) == max { 1 } else { 0 };
                    let c = if (byte | 0b11101111) == max { 1 } else { 0 };
                    let d = if (byte | 0b11110111) == max { 1 } else { 0 };
                    let e = if (byte | 0b11111011) == max { 1 } else { 0 };
                    let f = if (byte | 0b11111101) == max { 1 } else { 0 };
                    let g = if (byte | 0b11111110) == max { 1 } else { 0 };
                    let mut exp = bitvec![a, b, c, d, e, f, g];
                    total_exp.append(&mut exp);
                    // out of the second byte the first 7 bits are part of the
                    // exponent, and the last bit if part of the significand
                    let byte_2 = buffer[1];
                    let a = if (byte_2 | 0b01111111) == max { 1 } else { 0 };
                    let b = if (byte_2 | 0b10111111) == max { 1 } else { 0 };
                    let c = if (byte_2 | 0b11011111) == max { 1 } else { 0 };
                    let d = if (byte_2 | 0b11101111) == max { 1 } else { 0 };
                    let e = if (byte_2 | 0b11110111) == max { 1 } else { 0 };
                    let f = if (byte_2 | 0b11111011) == max { 1 } else { 0 };
                    let g = if (byte_2 | 0b11111101) == max { 1 } else { 0 };
                    let mut exp_cont = bitvec![a, b, c, d, e, f, g];
                    total_exp.append(&mut exp_cont);
                    let h = if (byte_2 | 0b11111110) == max { 1 } else { 0 };
                    // Start a new vec for 113bit significand. Since this
                    // version of significand is not offset, we pad it with only
                    // `0`
                    let mut sig = bitvec![0, h];
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

        let exp = Exponent(total_exp);
        let sig = Significand(total_sig);

        let dec128 = match combination_field {
            CombinationField::Finite => Decimal128 {
                sign,
                exponent: Some(exp),
                significand: Some(sig),
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
    pub fn to_string(&self) -> &str {
        unimplemented!();
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
