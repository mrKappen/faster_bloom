#![warn(rust_2018_idioms, missing_docs, dead_code)]
//! Bloom filter implementation in rust
//! A bloom filter will tell you with 100 % certainty if something is NOT in the
//! set. It won't tell you whether something is present

use rand::RngExt;
use siphasher::sip128::SipHasher;
use std::hash::{Hash, Hasher};

/// Bloom filter struct
pub struct BloomFilter {
    bits: Vec<bool>,
    hashers: Vec<SipHasher>,
    capacity: u128,
    error: f32,
}

impl BloomFilter {
    /// create bloom filter object
    pub fn new(capacity: u128, error: f32) -> Self {
        if error == 0f32 {
            panic!("The error threshold cannot be 0")
        }

        let m = get_m(error, capacity);
        let k = get_k(error);
        let bits = vec![false; m as usize];
        let mut hashers = Vec::new();
        let mut rng = rand::rng();
        for _ in 0..k {
            let key0: u64 = rng.random();
            let key1: u64 = rng.random();
            let hasher = SipHasher::new_with_keys(key0, key1);
            hashers.push(hasher);
        }

        Self {
            bits,
            hashers,
            capacity,
            error,
        }
    }

    /// insert item into bloom filter
    pub fn insert<T: Hash>(&mut self, item: T) {
        let indices = self.get_indices(item);
        // set flags to true
        for index in indices {
            self.bits[index as usize] = true;
        }
    }

    /// check if item is likely present
    pub fn is_present<T: Hash>(&self, item: T) -> bool {
        let indices = self.get_indices(item);
        for index in indices {
            if self.bits[index as usize] == false {
                return false;
            }
        }
        true
    }

    fn get_indices<T: Hash>(&self, item: T) -> Vec<u64> {
        let mut indices: Vec<u64> = Vec::new();
        let k: u64 = self.hashers.len() as u64;
        for h in &self.hashers {
            let mut hasher = h.clone();
            item.hash(&mut hasher);

            let index = hasher.finish() % k;
            indices.push(index)
        }

        indices
    }
}

fn get_m(error: f32, capacity: u128) -> u128 {
    let ln_e = error.ln();
    let denomenator = (2f32.ln()).powf(2f32);
    let m: f32 = -1f32 * (((capacity as f32) * ln_e) / denomenator);
    m.ceil() as u128
}

fn get_k(error: f32) -> u128 {
    (-1f32 * error.log2()).ceil() as u128
}
