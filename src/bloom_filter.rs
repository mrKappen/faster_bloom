use rand::RngExt;
use siphasher::sip128::SipHasher;
use std::hash::{Hash, Hasher};

///Error code
#[derive(Debug)]
pub enum ErrCode {
    /// filter Full
    Full,
    /// Bad input provided
    BadInput,
}

/// Bloom filter struct
pub struct BloomFilter {
    bits: Vec<bool>,
    hashers: Vec<SipHasher>,
    capacity: u128,
    error_tolerance: f32,
    num_inserts: u128,
}

impl BloomFilter {
    /// create bloom filter object
    pub fn new(capacity: u128, error_tolerance: f32) -> Result<Self, ErrCode> {
        if error_tolerance <= 0f32 || error_tolerance > 1.0 {
            return Err(ErrCode::BadInput);
        }

        if capacity == 0 {
            return Err(ErrCode::BadInput);
        }

        let m = get_m(error_tolerance, capacity);
        let k = get_k(error_tolerance);

        if k == 0 {
            return Err(ErrCode::BadInput);
        }

        let bits = vec![false; m as usize];
        let mut hashers = Vec::new();
        let mut rng = rand::rng();
        for _ in 0..k {
            let key0: u64 = rng.random();
            let key1: u64 = rng.random();
            let hasher = SipHasher::new_with_keys(key0, key1);
            hashers.push(hasher);
        }

        Ok(Self {
            bits,
            hashers,
            capacity,
            error_tolerance,
            num_inserts: 0,
        })
    }

    /// insert item into bloom filter
    pub fn insert<T: Hash>(&mut self, item: T) -> Result<(), ErrCode> {
        if self.get_num_inserts() >= self.get_capacity() {
            return Err(ErrCode::Full);
        }
        let indices = self.get_indices(item);
        // set flags to true
        for index in indices {
            self.bits[index as usize] = true;
        }
        self.num_inserts += 1;
        Ok(())
    }

    /// check if item is likely present
    pub fn is_present<T: Hash>(&self, item: T) -> bool {
        let indices = self.get_indices(item);
        for index in indices {
            if !self.bits[index as usize] {
                return false;
            }
        }
        true
    }

    fn get_indices<T: Hash>(&self, item: T) -> Vec<u64> {
        let mut indices: Vec<u64> = Vec::new();
        let k: u64 = self.bits.len() as u64;
        for h in &self.hashers {
            let mut hasher = *h;
            item.hash(&mut hasher);

            let index = hasher.finish() % k;
            indices.push(index)
        }

        indices
    }

    /// get error tolerance
    pub fn get_error_tolerance(&self) -> f32 {
        self.error_tolerance
    }

    /// get configured capacity
    pub fn get_capacity(&self) -> u128 {
        self.capacity
    }

    /// get number of inserts
    pub fn get_num_inserts(&self) -> u128 {
        self.num_inserts
    }
}

/**
 * m = [-n * ln (error)] / (ln(2) ^ 2)
 */
fn get_m(error: f32, capacity: u128) -> u128 {
    let ln_e = error.ln();
    let denomenator = (2f32.ln()).powf(2f32);
    let m: f32 = -(((capacity as f32) * ln_e) / denomenator);
    m.ceil() as u128
}

fn get_k(error: f32) -> u128 {
    (-error.log2()).ceil() as u128
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_not_present() {
        let mut bf = BloomFilter::new(1000, 0.01).unwrap();
        assert_eq!(bf.get_error_tolerance(), 0.01);
        assert_eq!(bf.get_capacity(), 1000);

        let key = "hello world";
        bf.insert(key).unwrap();
        assert!(!bf.is_present("good bye"));
        assert!(bf.is_present(key));
    }

    #[test]
    fn test_get_m() {
        assert!(get_m(0.01, 1000) == 9586);
        assert!(get_m(0.00001, 1000) == 23963);
        assert!(get_m(0.00001, 10) == 240);
    }

    #[test]
    fn test_get_k() {
        assert_eq!(get_k(0.01), 7);
        assert_eq!(get_k(0.99), 1);
    }

    #[test]
    fn test_new_creates_filter() {
        let bf = BloomFilter::new(1000, 0.01).unwrap();
        assert_eq!(bf.get_error_tolerance(), 0.01);
        assert_eq!(bf.get_capacity(), 1000);
    }

    #[test]
    fn test_new_err_on_zero_capacity() {
        assert!(BloomFilter::new(0, 0.01).is_err());
    }

    #[test]
    fn test_new_err_on_zero_error() {
        assert!(BloomFilter::new(100, 0.0).is_err());
    }

    #[test]
    fn test_new_err_on_negative_error() {
        assert!(BloomFilter::new(100, -0.5).is_err());
    }

    #[test]
    fn test_new_err_on_error_greater_than_one() {
        assert!(BloomFilter::new(100, 1.5).is_err());
    }

    #[test]
    fn test_new_err_on_error_tolerance_one() {
        // get_k(1.0) = ceil(-log2(1.0)) = 0, which triggers the k == 0 check
        assert!(BloomFilter::new(100, 1.0).is_err());
    }

    #[test]
    fn test_insert_and_check_present() {
        let mut bf = BloomFilter::new(1000, 0.01).unwrap();
        let items = vec!["apple", "banana", "cherry", "date", "elderberry"];
        for item in &items {
            bf.insert(item).unwrap();
        }
        for item in &items {
            assert!(bf.is_present(item), "Expected '{}' to be present", item);
        }
    }

    #[test]
    fn test_no_false_negatives() {
        let mut bf = BloomFilter::new(1000, 0.01).unwrap();
        for i in 0..500 {
            bf.insert(i).unwrap();
        }
        for i in 0..500 {
            assert!(
                bf.is_present(i),
                "False negative for item {}: bloom filters must never have false negatives",
                i
            );
        }
    }

    #[test]
    fn test_empty_filter_reports_not_present() {
        let bf = BloomFilter::new(1000, 0.01).unwrap();
        assert!(!bf.is_present("hello"));
        assert!(!bf.is_present(42));
        assert!(!bf.is_present(vec![1, 2, 3]));
        assert!(!bf.is_present(""));
    }

    #[test]
    fn test_not_present_for_missing_items() {
        let mut bf = BloomFilter::new(1000, 0.05).unwrap();
        for i in 0..100 {
            bf.insert(format!("inserted_{}", i)).unwrap();
        }
        let mut false_positives = 0;
        for i in 0..1000 {
            if bf.is_present(format!("missing_{}", i)) {
                false_positives += 1;
            }
        }
        // Allow a generous margin above the 5% theoretical rate to avoid flaky tests
        let threshold = 200;
        assert!(
            false_positives < threshold,
            "Too many false positives: {} out of 1000 (threshold: {})",
            false_positives,
            threshold
        );
    }

    #[test]
    fn test_insert_different_types() {
        let mut int_filter = BloomFilter::new(500, 0.01).unwrap();
        for i in 0u64..10 {
            int_filter.insert(i).unwrap();
        }
        for i in 0u64..10 {
            assert!(int_filter.is_present(i));
        }

        let mut str_filter = BloomFilter::new(500, 0.01).unwrap();
        let words = vec!["rust", "bloom", "filter", "hash"];
        for w in &words {
            str_filter.insert(w).unwrap();
        }
        for w in &words {
            assert!(str_filter.is_present(w));
        }
    }

    #[test]
    fn test_duplicate_insert() {
        let mut bf = BloomFilter::new(100, 0.01).unwrap();
        bf.insert("duplicate").unwrap();
        bf.insert("duplicate").unwrap();
        bf.insert("duplicate").unwrap();
        assert!(bf.is_present("duplicate"));
    }
}
