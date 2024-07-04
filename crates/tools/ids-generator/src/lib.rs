#![feature(portable_simd)]
mod bitset;

use std::{
	ops::Range,
	simd::{i32x16, num::SimdInt, Simd},
	sync::Arc,
};

use ahash::AHashSet;
use bitset::Bitset;
use parking_lot::Mutex;
use ring::rand::{SecureRandom, SystemRandom};

pub const IDS_POOL_SIZE: usize = 1024;

/// Generates unique IDs within a specified range.
#[derive(Clone)]
pub struct IdsGenerator {
	ids: Arc<Mutex<Vec<i32>>>,
	container: Arc<Mutex<IdContainer>>,
	simd: (Simd<i32, 16>, Simd<i32, 16>),
}

impl IdsGenerator {
	/// Creates a new `IdsGenerator`.
	///
	/// # Arguments
	///
	/// * `range` - A range of integers within which IDs will be generated.
	/// * `in_use_ids` - A vector of IDs that are already in use and should not be generated.
	///
	/// # Returns
	///
	/// A new instance of `IdsGenerator`.
	pub fn new(range: Range<i32>, in_use_ids: Vec<i32>) -> Self {
		let valid_range = range.end - range.start;
		let container = IdContainer::new(range.clone(), in_use_ids, valid_range);

		let range_start_simd = Simd::splat(range.start);
		let valid_range_simd = Simd::splat(valid_range);

		let generator = IdsGenerator {
			ids: Arc::new(Mutex::new(Vec::with_capacity(IDS_POOL_SIZE))),
			container: Arc::new(Mutex::new(container)),
			simd: (range_start_simd, valid_range_simd),
		};

		{
			let mut ids = generator.ids.lock();
			generator.refill(&mut ids);
		}

		generator
	}

	/// Returns the next available ID.
	///
	/// # Returns
	///
	/// The next available unique ID.
	///
	/// # Panics
	///
	/// Panics if no unique ID can be generated.
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

	pub fn container_type(&self) -> ContainerType {
		self.container.lock().container_type()
	}

	/// Refills the pool of available IDs.
	///
	/// # Arguments
	///
	/// * `ids` - A mutable reference to a vector of IDs to be refilled.
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

		for chunk in buf.chunks_exact(16) {
			let nums = i32x16::from_slice(chunk).saturating_abs();
			let ids_simd = self.simd.0 + (nums % self.simd.1);

			for i in 0..ids_simd.len() {
				let id = ids_simd[i];

				if container.insert(id) {
					ids.push(id);
				}
			}
		}
	}
}

/// Container Type
#[derive(Debug, PartialEq)]
pub enum ContainerType {
	HashSet,
	BitSet,
}

/// Container for managing used and available IDs.

enum IdContainer {
	HashSet(AHashSet<i32>, Range<i32>, usize),
	BitSet(Bitset),
}

impl IdContainer {
	/// Creates a new `IdContainer`.
	///
	/// # Arguments
	///
	/// * `range` - The range of valid IDs.
	/// * `in_use_ids` - A vector of IDs already in use.
	/// * `valid_range` - The range of valid IDs.
	///
	/// # Returns
	///
	/// A new instance of `IdContainer`.
	pub fn new(range: Range<i32>, in_use_ids: Vec<i32>, valid_range: i32) -> Self {
		let threshold = (valid_range as usize + 7) / 8;

		if in_use_ids.len() * 9 > threshold {
			let mut bitset = Bitset::new(range.clone());

			for id in in_use_ids {
				unsafe {
					bitset.insert(id);
				}
			}

			IdContainer::BitSet(bitset)
		} else {
			let mut hashset = AHashSet::with_capacity(in_use_ids.len());

			for id in in_use_ids {
				hashset.insert(id);
			}

			IdContainer::HashSet(hashset, range, threshold)
		}
	}

	/// Checks if the number of used IDs exceeds a threshold.
	#[inline]
	pub fn check_threshold(&mut self) {
		if let IdContainer::HashSet(ref hashset, range, threshold) = self {
			if hashset.len() * 9 > *threshold {
				let mut bitset = Bitset::new(range.clone());

				for id in hashset {
					unsafe {
						bitset.insert(*id);
					}
				}

				*self = IdContainer::BitSet(bitset)
			}
		}
	}

	/// Inserts a new ID into the container.
	///
	/// # Arguments
	///
	/// * `id` - The ID to insert.
	///
	/// # Returns
	///
	/// `true` if the ID was successfully inserted, `false` otherwise.
	#[inline]
	pub fn insert(&mut self, id: i32) -> bool {
		match self {
			IdContainer::BitSet(bitset) => unsafe { bitset.insert(id) },
			IdContainer::HashSet(hashset, _, _) => hashset.insert(id),
		}
	}

	pub fn container_type(&self) -> ContainerType {
		match self {
			IdContainer::HashSet(..) => ContainerType::HashSet,
			IdContainer::BitSet(..) => ContainerType::BitSet,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_id_generator_creation() {
		let range = 0..100000;
		let in_use_ids = vec![1, 2, 3];
		let generator = IdsGenerator::new(range, in_use_ids);

		assert!(!generator.ids.lock().is_empty());
	}

	#[test]
	fn test_id_generation() {
		let range = 0..100;
		let in_use_ids = vec![1, 2, 3];
		let generator = IdsGenerator::new(range, in_use_ids);

		let id = generator.next();
		assert!((0..100).contains(&id));
	}

	#[test]
	fn test_unique_ids() {
		let range = 0..1000;
		let in_use_ids = vec![1, 2, 3];
		let generator = IdsGenerator::new(range, in_use_ids);

		let mut ids = std::collections::HashSet::new();
		for _ in 0..100 {
			let id = generator.next();
			assert!(!ids.contains(&id));
			ids.insert(id);
		}
	}
}
