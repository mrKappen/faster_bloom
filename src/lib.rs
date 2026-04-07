#![warn(rust_2018_idioms, missing_docs, dead_code)]
//! A bloom filter will tell you with 100 % certainty if something is NOT in the
//! set. It won't tell you whether something is present

/// Standard bloom filter
pub mod bloom_filter;

/// Bloom filter that automatically scales as new items are added
pub mod scalable_bloom_filter;
