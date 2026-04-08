# bloom_filter

A Rust implementation of [Bloom filters](https://en.wikipedia.org/wiki/Bloom_filter) — space-efficient probabilistic data structures for set membership testing.

This crate provides two variants:

- **`BloomFilter`** — A standard, fixed-capacity bloom filter.
- **`ScalableBloomFilter`** — A bloom filter that automatically grows by adding new internal filters as items are inserted, while maintaining the desired false positive rate.

## How It Works

A bloom filter can tell you with **100% certainty** whether an item is **not** in the set. However, it can only tell you that an item is **probably** in the set — there is a configurable probability of false positives. There are **never** false negatives.

Under the hood, the filter maintains a bit array and a set of independent hash functions (using [SipHash 1-3](https://docs.rs/siphasher) with random keys). When an item is inserted, it is hashed by each function and the corresponding bits are set. When checking membership, the filter verifies that all corresponding bits are set.

### Scalable Bloom Filter

The `ScalableBloomFilter` wraps multiple `BloomFilter` instances in a chain. When the current filter reaches its capacity, a new filter is appended with:

- **Larger capacity** — scaled proportionally to the filter's position in the chain.
- **Tighter error tolerance** — divided by the filter's position, so the combined false positive rate across all filters stays bounded.

A membership query checks all filters in the chain and returns `true` if any of them reports the item as present.

## Installation

Add `bloom_filter` to your `Cargo.toml`:

```toml
[dependencies]
bloom_filter = "0.1.0"
```

## Usage

### Standard Bloom Filter

```rust
use bloom_filter::bloom_filter::BloomFilter;

// Create a filter with capacity for 10,000 items and a 1% false positive rate
let mut bf = BloomFilter::new(10_000, 0.01);

// Insert items (anything that implements Hash)
bf.insert("hello");
bf.insert(42u64);

// Check membership
assert!(bf.is_present("hello"));   // true — was inserted
assert!(!bf.is_present("world"));  // false — definitely not in the set
```

The `insert` method returns a `StatusCode`:

- `StatusCode::SUCCESS` — the item was inserted.
- `StatusCode::FULL` — the filter has reached capacity. The item was **not** inserted.

### Scalable Bloom Filter

```rust
use bloom_filter::scalable_bloom_filter::ScalableBloomFilter;

// Create a scalable filter with a 1% false positive rate.
// It starts with an internal capacity of 10,000 and grows automatically.
let mut sbf = ScalableBloomFilter::new(0.01);

// Insert items — the filter scales as needed
for i in 0..50_000u64 {
    sbf.insert(&i);
}

// All inserted items will be found (no false negatives, ever)
assert!(sbf.is_present(0u64));
assert!(sbf.is_present(49_999u64));
```

## API Reference

### `BloomFilter`

| Method | Description |
|--------|-------------|
| `BloomFilter::new(capacity, error_tolerance)` | Create a new filter. Panics if `capacity` is 0, `error_tolerance` is ≤ 0 or > 1, or the resulting hash count is 0. |
| `.insert(item)` | Insert an item. Returns `StatusCode::SUCCESS` or `StatusCode::FULL`. |
| `.is_present(item)` | Check if an item is probably in the set. `false` means definitely absent. |
| `.get_capacity()` | Returns the configured capacity. |
| `.get_error_tolerance()` | Returns the configured false positive rate. |
| `.get_num_inserts()` | Returns the number of insertions performed. |

### `ScalableBloomFilter`

| Method | Description |
|--------|-------------|
| `ScalableBloomFilter::new(error_tolerance)` | Create a new scalable filter with the given false positive rate. |
| `.insert(&item)` | Insert an item. Automatically creates a new internal filter if the current one is full. |
| `.is_present(item)` | Check if an item is probably present in any of the internal filters. |

## Configuration

The two parameters that control a bloom filter's behavior are:

- **`capacity`** — The maximum number of items the filter is designed to hold. Inserting more items than this will degrade the false positive rate (in a standard filter) or trigger scaling (in a scalable filter).
- **`error_tolerance`** — The desired false positive probability, e.g., `0.01` for 1%. Lower values require more memory but produce fewer false positives.

These parameters determine:

- **`m`** (bit array size): `m = ⌈-n × ln(ε) / (ln 2)²⌉`
- **`k`** (number of hash functions): `k = ⌈-log₂(ε)⌉`

Where `n` is the capacity and `ε` is the error tolerance.

## License

This project is licensed under the [MIT License](LICENSE).