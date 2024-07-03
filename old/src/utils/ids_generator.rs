use std::{
    ops::Range,
    simd::{i32x16, num::SimdInt, Simd},
    sync::Arc,
};

use ahash::AHashSet;
use parking_lot::Mutex;
use ring::rand::{SecureRandom, SystemRandom};

use crate::config::constants::IDS_POOL_SIZE;

use super::bitset::Bitset;

/// `IdContainer` holds IDs using either a `HashSet` or a `Bitset`.
/// Automatically switches based on a threshold for efficient storage.
pub enum IdContainer {
    HashSet(AHashSet<i32>, Range<i32>, usize), // Range & Threshold
    Bitset(Bitset),
}

impl IdContainer {
    /// Creates a new `IdContainer` based on the range and existing IDs.
    ///
    /// # Arguments
    ///
    /// * `range` - The range from which IDs are generated.
    /// * `in_use_ids` - IDs that are currently in use.
    /// * `threshold` - The threshold to determine the storage type.
    pub fn new(range: Range<i32>, in_use_ids: Vec<i32>, valid_range: i32) -> Self {
        let threshold = (valid_range as usize + 7) / 8;

        if in_use_ids.len() * 9 > threshold {
            let mut bitset = Bitset::new(range.clone());

            for id in in_use_ids {
                unsafe {
                    bitset.set(id);
                }
            }

            IdContainer::Bitset(bitset)
        } else {
            let mut hashset = AHashSet::with_capacity(in_use_ids.len());

            for id in in_use_ids {
                hashset.insert(id);
            }

            IdContainer::HashSet(hashset, range, threshold)
        }
    }

    /// Checks if the threshold is exceeded and converts to `Bitset` if needed.
    pub fn check_threshold(&mut self) {
        if let IdContainer::HashSet(ref hashset, range, threshold) = self {
            if hashset.len() * 9 > *threshold {
                let mut bitset = Bitset::new(range.clone());

                for id in hashset {
                    unsafe {
                        bitset.set(*id);
                    }
                }

                *self = IdContainer::Bitset(bitset)
            }
        }
    }

    /// Inserts an ID if it is not already present.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID to insert.
    ///
    /// # Returns
    ///
    /// `true` if the ID was inserted, `false` if it was already present.
    pub fn insert(&mut self, id: i32) -> bool {
        match self {
            IdContainer::Bitset(bitset) => unsafe {
                if !bitset.check(id) {
                    bitset.set(id);
                    true
                } else {
                    false
                }
            },

            IdContainer::HashSet(hashset, _, _) => hashset.insert(id),
        }
    }
}

/// Struct for generating unique IDs within a specific range.
///
/// Maintains a buffer of available IDs to minimize on-demand generation. Automatically refills
/// when half of the buffer is consumed.
/// This implementation is only suitable for 1 server instance. Because it uses a `HashSet` or a `Bitset` to store IDs,
/// it is not suitable for distributed systems.
///
/// # Examples
///
/// ```
/// use crate::utils::IdsGenerator;
///
/// let generator = IdsGenerator::new(1..=100, Some(100));
///
/// // Generate an ID.
/// let id = generator.next();
/// assert!(id >= 1 && id <= 100);
/// ```
#[derive(Clone)]
pub struct IdsGenerator {
    ids: Arc<Mutex<Vec<i32>>>,
    container: Arc<Mutex<IdContainer>>,
    range: Range<i32>,
    valid_range: i32,
}

impl IdsGenerator {
    /// Creates a new `IdsGenerator`.
    ///
    /// # Arguments
    ///
    /// * `range` - The `RangeInclusive<i32>` from which IDs are generated.
    /// * `size` - Optional `usize` specifying the pool size. Uses a default if None.
    pub fn new(range: Range<i32>, in_use_ids: Vec<i32>) -> Self {
        let valid_range = range.end - range.start;
        let container = IdContainer::new(range.clone(), in_use_ids, valid_range);

        let generator = IdsGenerator {
            ids: Arc::new(Mutex::new(Vec::with_capacity(IDS_POOL_SIZE))),
            container: Arc::new(Mutex::new(container)),
            range,
            valid_range,
        };

        {
            let mut ids = generator.ids.lock();
            generator.refill(&mut ids);
        }

        generator
    }

    /// Refills the ID pool by generating new unique IDs.
    ///
    /// This method generates unique IDs within the specified range, ensuring
    /// that each ID is not already present in the pool or the underlying
    /// container. It uses SIMD for efficient random number generation and
    /// checks if the container needs to be converted based on a threshold.
    ///
    /// Note: `refill` is designed to be called internally to maintain a
    /// sufficient supply of unique IDs.
    fn refill(&self, ids: &mut Vec<i32>) {
        let rng = SystemRandom::new();
        let mut buf = [0i32; IDS_POOL_SIZE];
        let mut container = self.container.lock();

        let byte_buf = unsafe {
            std::slice::from_raw_parts_mut(
                buf.as_mut_ptr() as *mut u8,
                buf.len() * size_of::<i32>(),
            )
        };

        rng.fill(byte_buf).expect("Failed to generate random byte");

        container.check_threshold();

        let valid_range_simd = Simd::splat(self.valid_range);
        let range_start_simd = Simd::splat(self.range.start);

        for chunk in buf.chunks_exact(16) {
            let nums = i32x16::from_slice(chunk).saturating_abs();
            let ids_simd = range_start_simd + (nums % valid_range_simd);

            for i in 0..ids_simd.len() {
                let id = ids_simd[i];

                if container.insert(id) {
                    ids.push(id);
                }
            }
        }
    }

    /// Returns a unique ID.
    ///
    /// Refills the pool automatically if more than half is consumed.
    ///
    /// # Returns
    ///
    /// Returns an `i32` as the generated ID.
    pub fn next(&self) -> i32 {
        let mut ids = self.ids.lock();

        match ids.pop() {
            Some(id) => id,
            None => {
                self.refill(&mut ids);
                ids.pop().unwrap_or_else(|| {
                    panic!("Failed to generate a unique ID: No more unique IDs available")
                })
            }
        }
    }
}
