use std::ops::Range;

/// A structure to manage a set of bits efficiently.
pub struct Bitset {
    bits: Vec<u8>,
    range: Range<i32>,
}

impl Bitset {
    /// Creates a new Bitset for the given range.
    ///
    /// # Arguments
    ///
    /// * `range` - A range of i32 values.
    ///
    /// # Panics
    ///
    /// This function does not panic, but using an extremely large range may lead
    /// to excessive memory usage.
    pub fn new(range: Range<i32>) -> Self {
        let size = ((range.end - range.start) as usize + 7) / 8;

        Self {
            bits: vec![0; size],
            range,
        }
    }

    /// Checks if the bit corresponding to the given value is set.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to check the bit for.
    ///
    /// # Returns
    ///
    /// `true` if the bit is set, `false` otherwise.
    ///
    /// # Safety
    ///
    /// This function is marked as unsafe because it uses unchecked indexing
    /// (`get_unchecked`). It assumes that the value is always within the
    /// range provided during the creation of the Bitset. The parent structure
    /// (IdsGenerator) must ensure that only valid values are used.
    pub unsafe fn check(&self, value: i32) -> bool {
        let index = (value - self.range.start) as usize;
        let byte = self.bits.get_unchecked(index / 8);
        (*byte & (1 << (index % 8))) != 0
    }

    /// Sets the bit corresponding to the given value.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to set the bit for.
    ///
    /// # Safety
    ///
    /// This function is marked as unsafe because it uses unchecked indexing
    /// (`get_unchecked_mut`). It assumes that the value is always within the
    /// range provided during the creation of the Bitset. The parent structure
    /// (IdsGenerator) must ensure that only valid values are used.
    pub unsafe fn set(&mut self, value: i32) {
        let index = (value - self.range.start) as usize;
        let byte = self.bits.get_unchecked_mut(index / 8);
        *byte |= 1 << (index % 8);
    }

    /// Clears the bit corresponding to the given value.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to clear the bit for.
    ///
    /// # Safety
    ///
    /// This function is marked as unsafe because it uses unchecked indexing
    /// (`get_unchecked_mut`). It assumes that the value is always within the
    /// range provided during the creation of the Bitset. The parent structure
    /// (IdsGenerator) must ensure that only valid values are used.
    #[allow(unused)]
    pub unsafe fn clear(&mut self, value: i32) {
        let index = (value - self.range.start) as usize;
        let byte = self.bits.get_unchecked_mut(index / 8);
        *byte &= !(1 << (index % 8));
    }
}
