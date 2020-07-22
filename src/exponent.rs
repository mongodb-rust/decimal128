#[derive(Clone, PartialEq, PartialOrd)]
// where msb0 is big endian.
pub(crate) struct Exponent {
    inner: u16,
}
/// Exponent is a 14-bit portion of decimal128 that follows the sign bit. Here we
/// are storing it as a 16-bit BitVec that can be later converted to a u16.
impl Exponent {
    pub(crate) fn new() -> Self {
        Exponent { inner: 0 }
    }

    // compare current exponent value with exponent bias (largest possible
    // exponent value)
    // TODO: check if 6176 (exponent bias) can be stored as u16
    pub fn to_adjusted(&self) -> i16 {
        // NOTE: this could potentially panic if self.inner is larger than max i16
        self.inner as i16 - 6176 as i16
    }
}
