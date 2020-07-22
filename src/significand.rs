#[derive(Clone, PartialEq, PartialOrd)]
// where msb0 is big endian.
pub(crate) struct Significand {
    inner: u128,
}

/// Significand is a padded 111- or 113-bit coefficient. We are storing it as a
/// 128-bit BitVec with the padded difference. This can be converted to a u128.
impl Significand {
    pub(crate) fn new() -> Self {
        Significand { inner: 0 }
    }

    // max number from Decimal128 spec
    // TODO: document usage better
    pub(crate) fn max_value() -> u128 {
        u128::from_str_radix("9999999999999999999999999999999999", 10).unwrap()
    }

    // count the number of digits in the significand. This method first converts
    // significand BitVec into a u128 number, then converts it to string to
    // count characters and collects them in a vec to look at the vec's length.
    //
    // We return a u16 number of digits, as it's easier to compare to the
    // exponent since that's also stored as a u16.
    // TODO: use a logarithm method for this to remove intermediate allocation in as_digi_vec
    pub(crate) fn count_digits(&self) -> i16 {
        self.as_digit_vec().len() as i16
    }

    pub(crate) fn as_digit_vec(&self) -> Vec<u32> {
        let digits: Vec<u32> = self
            .inner
            .to_string()
            .chars()
            .map(|c| c.to_digit(10).unwrap())
            .collect();
        return digits;
    }
}
