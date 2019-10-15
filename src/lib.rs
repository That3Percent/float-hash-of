use hash_of::*;
use std::borrow::Borrow;
use std::hash::Hash;
use std::marker::PhantomData;
use std::num::NonZeroU64;

/// Takes a 64 bit hash, and makes a primitive 63 bit hash using a double.
/// This allows for easy comparison without keeping making heap allocations in JavaScript (eg: string) or requiring low entropy (int)
#[derive(Eq, PartialEq, Debug, Hash)]
pub struct FloatHashOf<T> {
    // There's no such thing as a NonZeroF64, so store as NonZeroU64 and transmute when necessary.
    // This let's us store it in Option without increasing the size.
    hash: NonZeroU64,
    _marker: PhantomData<*const T>, // Indicate we do not own T
}

// Manually implementing Copy/Clone because they are not automatically derived
// if T does not equal Copy/Clone
impl<T> Clone for FloatHashOf<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for FloatHashOf<T> {}

impl<T> FloatHashOf<T> {
    #[inline]
    pub fn into_inner(self) -> f64 {
        f64::from_bits(self.hash.get())
    }
}

// We want to avoid NaN (unexpected equality rules), subnormals (they may be slow?),
// and +-0 (inconsistent equality rules, useful as a null hash)
// so ensure there is at least one each of 0 and 1 in the exponent to make those cases impossible.
// This also rules out +-INF
fn hash_63_bits(hash: u64) -> u64 {
    #![allow(clippy::inconsistent_digit_grouping)] // Grouping matches 64bit IEEE 754 float

    // TODO: This assumes little-endian, but we could cgf the big-endian format in
    const EXP_2: u64 = 0b0_11000000000_0000000000000000000000000000000000000000000000000000;
    const EXP_1: u64 = 0b0_10000000000_0000000000000000000000000000000000000000000000000000;
    const EXP_0: u64 = 0b0_00000000000_0000000000000000000000000000000000000000000000000000;

    match hash & EXP_2 {
        EXP_0 => hash | EXP_1,
        EXP_2 => hash ^ EXP_1,
        _ => hash,
    }
}

pub fn hash_u64_to_f64(hash: u64) -> f64 {
    f64::from_bits(hash_63_bits(hash))
}

impl<T> From<HashOf<T>> for FloatHashOf<T> {
    fn from(hash: HashOf<T>) -> Self {
        let mut hash = hash.to_inner();
        hash = hash_63_bits(hash);

        Self {
            hash: unsafe { NonZeroU64::new_unchecked(hash) },
            _marker: PhantomData,
        }
    }
}

// Example types to explain the confusing signature...
// T: str
// Q: String
impl<T: Hash + ?Sized, Q: Borrow<T>> From<&T> for FloatHashOf<Q> {
    fn from(value: &T) -> Self {
        let hash = HashOf::<Q>::from(value);
        hash.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn test_cases() -> HashSet<u64> {
        let mut cases = HashSet::new();
        cases.insert(0);
        for start in 0..16 {
            for shift in 0..(0u64.count_zeros()) {
                let case = start << shift;
                cases.insert(case);
            }
        }
        cases
    }

    #[test]
    fn no_invalid_values() {
        let cases = test_cases();
        for &case in cases.iter() {
            let result = hash_u64_to_f64(case);
            assert!(result != 0.);
            assert!(!result.is_nan());
        }
    }
}
