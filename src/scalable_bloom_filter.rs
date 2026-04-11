use crate::bloom_filter::{BloomFilter, ErrCode};
use std::hash::Hash;

/// Define a bloom filter that automatically scales
/// as new items are added while maintaining the required error threshold
pub struct ScalableBloomFilter {
    error_tolerance: f32,
    filters: Vec<BloomFilter>,
}

const STARTING_CAPACITY: u128 = 10_000;
impl ScalableBloomFilter {
    /// create scalable bloom filter
    pub fn new(error_tolerance: f32) -> Result<Self, ErrCode> {
        let initial_filter = BloomFilter::new(STARTING_CAPACITY, error_tolerance)?;
        let filters = vec![initial_filter];
        Ok(Self {
            error_tolerance,
            filters,
        })
    }

    /// insert item into scalable bloom filter
    pub fn insert<T: Hash>(&mut self, item: &T) -> Result<(), ErrCode> {
        let i = self.filters.len();
        let curr_filter = &mut self.filters[i - 1];
        match curr_filter.insert(item) {
            Ok(_) => Ok(()),
            Err(e) => match e {
                ErrCode::Full => {
                    let new_filter = BloomFilter::new(
                        STARTING_CAPACITY * ((i + 1) as u128),
                        self.error_tolerance / ((i + 1) as f32),
                    )?;
                    self.filters.push(new_filter);
                    self.insert(item)
                }
                _ => Err(e),
            },
        }
    }

    /// check if item is present scalable bloom filter
    pub fn is_present<T: Hash>(&self, item: T) -> bool {
        for filter in &self.filters {
            if filter.is_present(&item) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_scalable_filter() {
        let sbf = ScalableBloomFilter::new(0.01).unwrap();
        assert_eq!(sbf.error_tolerance, 0.01);
        assert_eq!(sbf.filters.len(), 1);
    }

    #[test]
    fn test_insert_and_check_present() {
        let mut sbf = ScalableBloomFilter::new(0.01).unwrap();
        let items = vec!["apple", "banana", "cherry", "date", "elderberry"];
        for item in &items {
            sbf.insert(item).unwrap();
        }
        for item in &items {
            assert!(sbf.is_present(item), "Expected '{}' to be present", item);
        }
    }

    #[test]
    fn test_empty_filter_reports_not_present() {
        let sbf = ScalableBloomFilter::new(0.01).unwrap();
        assert!(!sbf.is_present("hello"));
        assert!(!sbf.is_present(42));
        assert!(!sbf.is_present(""));
    }

    #[test]
    fn test_no_false_negatives() {
        let mut sbf = ScalableBloomFilter::new(0.01).unwrap();
        for i in 0..500 {
            sbf.insert(&i).unwrap();
        }
        for i in 0..500 {
            assert!(
                sbf.is_present(i),
                "False negative for item {}: bloom filters must never have false negatives",
                i
            );
        }
    }

    #[test]
    fn test_scales_beyond_starting_capacity() {
        let mut sbf = ScalableBloomFilter::new(0.01).unwrap();
        // Insert more than STARTING_CAPACITY (10,000) items to force scaling
        for i in 0..10_001u64 {
            sbf.insert(&i).unwrap();
        }
        assert!(
            sbf.filters.len() > 1,
            "Expected filter to scale beyond one inner filter, but had {}",
            sbf.filters.len()
        );
        // All inserted items must still be found (no false negatives across filters)
        for i in 0..10_001u64 {
            assert!(
                sbf.is_present(i),
                "False negative for item {} after scaling",
                i
            );
        }
    }

    #[test]
    fn test_second_filter_has_tighter_error_tolerance() {
        let mut sbf = ScalableBloomFilter::new(0.10).unwrap();
        // Fill up the first filter to force a second one
        for i in 0..10_001u64 {
            sbf.insert(&i).unwrap();
        }
        assert!(sbf.filters.len() >= 2);
        let first_error = sbf.filters[0].get_error_tolerance();
        let second_error = sbf.filters[1].get_error_tolerance();
        assert!(
            second_error < first_error,
            "Second filter error tolerance ({}) should be tighter than first ({})",
            second_error,
            first_error
        );
    }

    #[test]
    fn test_second_filter_has_larger_capacity() {
        let mut sbf = ScalableBloomFilter::new(0.10).unwrap();
        for i in 0..10_001u64 {
            sbf.insert(&i).unwrap();
        }
        assert!(sbf.filters.len() >= 2);
        let first_cap = sbf.filters[0].get_capacity();
        let second_cap = sbf.filters[1].get_capacity();
        assert!(
            second_cap > first_cap,
            "Second filter capacity ({}) should be larger than first ({})",
            second_cap,
            first_cap
        );
    }

    #[test]
    fn test_insert_returns_ok() {
        let mut sbf = ScalableBloomFilter::new(0.01).unwrap();
        let result = sbf.insert(&"test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_insert_different_types() {
        let mut sbf = ScalableBloomFilter::new(0.01).unwrap();
        sbf.insert(&42u64).unwrap();
        sbf.insert(&"hello").unwrap();
        sbf.insert(&vec![1, 2, 3]).unwrap();
        assert!(sbf.is_present(42u64));
        assert!(sbf.is_present("hello"));
        assert!(sbf.is_present(vec![1, 2, 3]));
    }

    #[test]
    fn test_duplicate_inserts_increment_count() {
        let mut sbf = ScalableBloomFilter::new(0.01).unwrap();
        sbf.insert(&"duplicate").unwrap();
        sbf.insert(&"duplicate").unwrap();
        sbf.insert(&"duplicate").unwrap();
        assert!(sbf.is_present("duplicate"));
        // Each insert increments the counter regardless of duplicates
        assert_eq!(sbf.filters[0].get_num_inserts(), 3);
    }

    #[test]
    fn test_not_present_for_missing_items() {
        let mut sbf = ScalableBloomFilter::new(0.05).unwrap();
        for i in 0..100 {
            sbf.insert(&format!("inserted_{}", i)).unwrap();
        }
        let mut false_positives = 0;
        for i in 0..1000 {
            if sbf.is_present(format!("missing_{}", i)) {
                false_positives += 1;
            }
        }
        let threshold = 200;
        assert!(
            false_positives < threshold,
            "Too many false positives: {} out of 1000 (threshold: {})",
            false_positives,
            threshold
        );
    }

    #[test]
    fn test_no_false_negatives_across_multiple_filters() {
        let mut sbf = ScalableBloomFilter::new(0.01).unwrap();
        // Insert enough to span multiple inner filters
        for i in 0..25_000u64 {
            sbf.insert(&i).unwrap();
        }
        assert!(
            sbf.filters.len() >= 2,
            "Expected multiple filters but got {}",
            sbf.filters.len()
        );
        for i in 0..25_000u64 {
            assert!(
                sbf.is_present(i),
                "False negative for {} across {} filters",
                i,
                sbf.filters.len()
            );
        }
    }

    #[test]
    fn test_single_item() {
        let mut sbf = ScalableBloomFilter::new(0.01).unwrap();
        sbf.insert(&"only_item").unwrap();
        assert!(sbf.is_present("only_item"));
        assert!(!sbf.is_present("other_item"));
    }
}
