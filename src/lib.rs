use byteorder::ReadBytesExt;
use failure::ensure;
use std::io::Cursor;

pub struct Exponent(u8, u8);
pub struct Coefficient(u8, u8, u8, u8);

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
            0b11111111 => CombinationField::NaN,
            0b11111011 => CombinationField::Infinity,
            _ => match byte | 0b1001111 {
                0b11111111 => {
                    let c = if (byte | 0b11101111) == max { 1 } else { 0 };
                    let d = if (byte | 0b11110111) == max { 1 } else { 0 };
                    let e = if (byte | 0b11111011) == max { 1 } else { 0 };
                    let exp = Exponent(c, d);
                    let coef = Coefficient(1, 0, 0, e);
                    CombinationField::Finite(exp, coef)
                }
                _ => {
                    let a = if (byte | 0b10111111) == max { 1 } else { 0 };
                    let b = if (byte | 0b11011111) == max { 1 } else { 0 };
                    let c = if (byte | 0b11101111) == max { 1 } else { 0 };
                    let d = if (byte | 0b11110111) == max { 1 } else { 0 };
                    let e = if (byte | 0b11111011) == max { 1 } else { 0 };
                    let exp = Exponent(a, b);
                    let coef = Coefficient(0, c, d, e);
                    CombinationField::Finite(exp, coef)
                }
            },
        };

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
