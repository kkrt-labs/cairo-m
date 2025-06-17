use std::collections::HashMap;

use stwo_prover::core::fields::qm31::QM31;

use super::io::MemEntry;

#[derive(Debug, Default, Clone)]
pub struct MemoryBoundaries {
    pub initial_memory: Vec<MemEntry>,
    pub final_memory: Vec<MemEntry>,
}

#[derive(Debug, Default)]
pub struct MemoryCache {
    clock_cache: HashMap<u32, u32>,
    initial_memory: HashMap<u32, [u32; 4]>,
    current_memory: HashMap<u32, [u32; 4]>,
}

impl MemoryCache {
    pub fn new() -> Self {
        Self {
            clock_cache: HashMap::new(),
            initial_memory: HashMap::new(),
            current_memory: HashMap::new(),
        }
    }

    pub fn push(&mut self, mem_entry: MemEntry, clock: u32) -> (u32, QM31, QM31, u32, u32) {
        let prev_clock = self
            .clock_cache
            .get(&mem_entry.addr)
            .copied()
            .unwrap_or_else(|| {
                self.initial_memory.insert(mem_entry.addr, mem_entry.val);
                0
            });
        self.clock_cache.insert(mem_entry.addr, clock);
        let old_value = self
            .current_memory
            .insert(mem_entry.addr, mem_entry.val)
            .unwrap_or([0, 0, 0, 0]);

        (
            mem_entry.addr,
            QM31::from_u32_unchecked(old_value[0], old_value[1], old_value[2], old_value[3]),
            QM31::from_u32_unchecked(
                mem_entry.val[0],
                mem_entry.val[1],
                mem_entry.val[2],
                mem_entry.val[3],
            ),
            prev_clock,
            clock,
        )
    }

    pub fn get_memory_boundaries(&self) -> MemoryBoundaries {
        MemoryBoundaries {
            initial_memory: self
                .initial_memory
                .iter()
                .map(|(&addr, &val)| MemEntry { addr, val })
                .collect(),
            final_memory: self
                .current_memory
                .iter()
                .map(|(&addr, &val)| MemEntry { addr, val })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_boundaries_default() {
        let boundaries = MemoryBoundaries::default();
        assert!(boundaries.initial_memory.is_empty());
        assert!(boundaries.final_memory.is_empty());
    }

    #[test]
    fn test_memory_cache_default() {
        let cache = MemoryCache::default();
        assert!(cache.clock_cache.is_empty());
        assert!(cache.initial_memory.is_empty());
        assert!(cache.current_memory.is_empty());
    }

    #[test]
    fn test_memory_cache_push_first_entry() {
        let mut cache = MemoryCache::new();
        let mem_entry = MemEntry {
            addr: 42,
            val: [1, 2, 3, 4],
        };
        let clock = 10;

        let (addr, old_value, new_value, prev_clock, current_clock) = cache.push(mem_entry, clock);

        assert_eq!(addr, 42);
        assert_eq!(old_value, QM31::from_u32_unchecked(0, 0, 0, 0));
        assert_eq!(new_value, QM31::from_u32_unchecked(1, 2, 3, 4));
        assert_eq!(prev_clock, 0);
        assert_eq!(current_clock, 10);

        // Check internal state
        assert_eq!(cache.clock_cache.get(&42), Some(&10));
        assert_eq!(cache.initial_memory.get(&42), Some(&[1, 2, 3, 4]));
        assert_eq!(cache.current_memory.get(&42), Some(&[1, 2, 3, 4]));
    }

    #[test]
    fn test_memory_cache_push_second_entry_same_address() {
        let mut cache = MemoryCache::new();
        let first_entry = MemEntry {
            addr: 42,
            val: [1, 2, 3, 4],
        };
        let second_entry = MemEntry {
            addr: 42,
            val: [5, 6, 7, 8],
        };

        // Push first entry
        cache.push(first_entry, 10);

        // Push second entry to same address
        let (addr, old_value, new_value, prev_clock, current_clock) = cache.push(second_entry, 20);

        assert_eq!(addr, 42);
        assert_eq!(old_value, QM31::from_u32_unchecked(1, 2, 3, 4));
        assert_eq!(new_value, QM31::from_u32_unchecked(5, 6, 7, 8));
        assert_eq!(prev_clock, 10);
        assert_eq!(current_clock, 20);

        // Check internal state
        assert_eq!(cache.clock_cache.get(&42), Some(&20));
        assert_eq!(cache.initial_memory.get(&42), Some(&[1, 2, 3, 4])); // Should remain unchanged
        assert_eq!(cache.current_memory.get(&42), Some(&[5, 6, 7, 8])); // Should be updated
    }

    #[test]
    fn test_memory_cache_push_different_addresses() {
        let mut cache = MemoryCache::new();
        let entry1 = MemEntry {
            addr: 42,
            val: [1, 2, 3, 4],
        };
        let entry2 = MemEntry {
            addr: 100,
            val: [5, 6, 7, 8],
        };

        cache.push(entry1, 10);
        let (addr, old_value, new_value, prev_clock, current_clock) = cache.push(entry2, 20);

        assert_eq!(addr, 100);
        assert_eq!(old_value, QM31::from_u32_unchecked(0, 0, 0, 0)); // New address, so old value is zero
        assert_eq!(new_value, QM31::from_u32_unchecked(5, 6, 7, 8));
        assert_eq!(prev_clock, 0); // New address, so previous clock is 0
        assert_eq!(current_clock, 20);

        // Check both entries exist
        assert_eq!(cache.clock_cache.len(), 2);
        assert_eq!(cache.initial_memory.len(), 2);
        assert_eq!(cache.current_memory.len(), 2);
    }

    #[test]
    fn test_memory_cache_multiple_updates_same_address() {
        let mut cache = MemoryCache::new();
        let addr = 42;

        // First update
        let entry1 = MemEntry {
            addr,
            val: [1, 2, 3, 4],
        };
        cache.push(entry1, 10);

        // Second update
        let entry2 = MemEntry {
            addr,
            val: [5, 6, 7, 8],
        };
        cache.push(entry2, 20);

        // Third update
        let entry3 = MemEntry {
            addr,
            val: [9, 10, 11, 12],
        };
        let (_, old_value, new_value, prev_clock, current_clock) = cache.push(entry3, 30);

        assert_eq!(old_value, QM31::from_u32_unchecked(5, 6, 7, 8));
        assert_eq!(new_value, QM31::from_u32_unchecked(9, 10, 11, 12));
        assert_eq!(prev_clock, 20);
        assert_eq!(current_clock, 30);

        // Initial memory should still be the first value
        assert_eq!(cache.initial_memory.get(&addr), Some(&[1, 2, 3, 4]));
        // Current memory should be the latest value
        assert_eq!(cache.current_memory.get(&addr), Some(&[9, 10, 11, 12]));
    }

    #[test]
    fn test_get_memory_boundaries_single_entry() {
        let mut cache = MemoryCache::new();
        let entry = MemEntry {
            addr: 42,
            val: [1, 2, 3, 4],
        };
        cache.push(entry, 10);

        let boundaries = cache.get_memory_boundaries();

        assert_eq!(boundaries.initial_memory.len(), 1);
        assert_eq!(boundaries.final_memory.len(), 1);
        assert_eq!(boundaries.initial_memory[0].addr, 42);
        assert_eq!(boundaries.initial_memory[0].val, [1, 2, 3, 4]);
        assert_eq!(boundaries.final_memory[0].addr, 42);
        assert_eq!(boundaries.final_memory[0].val, [1, 2, 3, 4]);
    }

    #[test]
    fn test_get_memory_boundaries_multiple_entries() {
        let mut cache = MemoryCache::new();

        // Add multiple entries to different addresses
        cache.push(
            MemEntry {
                addr: 42,
                val: [1, 2, 3, 4],
            },
            10,
        );
        cache.push(
            MemEntry {
                addr: 100,
                val: [5, 6, 7, 8],
            },
            20,
        );
        cache.push(
            MemEntry {
                addr: 200,
                val: [9, 10, 11, 12],
            },
            30,
        );

        let boundaries = cache.get_memory_boundaries();

        assert_eq!(boundaries.initial_memory.len(), 3);
        assert_eq!(boundaries.final_memory.len(), 3);

        // Sort for consistent comparison (HashMap iteration order is not guaranteed)
        let mut initial_sorted = boundaries.initial_memory;
        let mut final_sorted = boundaries.final_memory;
        initial_sorted.sort_by_key(|entry| entry.addr);
        final_sorted.sort_by_key(|entry| entry.addr);

        assert_eq!(initial_sorted[0].addr, 42);
        assert_eq!(initial_sorted[0].val, [1, 2, 3, 4]);
        assert_eq!(initial_sorted[1].addr, 100);
        assert_eq!(initial_sorted[1].val, [5, 6, 7, 8]);
        assert_eq!(initial_sorted[2].addr, 200);
        assert_eq!(initial_sorted[2].val, [9, 10, 11, 12]);
    }

    #[test]
    fn test_get_memory_boundaries_with_updates() {
        let mut cache = MemoryCache::new();

        // Add initial entry
        cache.push(
            MemEntry {
                addr: 42,
                val: [1, 2, 3, 4],
            },
            10,
        );
        // Update same address
        cache.push(
            MemEntry {
                addr: 42,
                val: [5, 6, 7, 8],
            },
            20,
        );
        // Add different address
        cache.push(
            MemEntry {
                addr: 100,
                val: [9, 10, 11, 12],
            },
            30,
        );

        let boundaries = cache.get_memory_boundaries();

        assert_eq!(boundaries.initial_memory.len(), 2);
        assert_eq!(boundaries.final_memory.len(), 2);

        // Sort for consistent comparison
        let mut initial_sorted = boundaries.initial_memory;
        let mut final_sorted = boundaries.final_memory;
        initial_sorted.sort_by_key(|entry| entry.addr);
        final_sorted.sort_by_key(|entry| entry.addr);

        // Initial memory should contain first values
        assert_eq!(initial_sorted[0].addr, 42);
        assert_eq!(initial_sorted[0].val, [1, 2, 3, 4]); // Initial value
        assert_eq!(initial_sorted[1].addr, 100);
        assert_eq!(initial_sorted[1].val, [9, 10, 11, 12]);

        // Final memory should contain current values
        assert_eq!(final_sorted[0].addr, 42);
        assert_eq!(final_sorted[0].val, [5, 6, 7, 8]); // Updated value
        assert_eq!(final_sorted[1].addr, 100);
        assert_eq!(final_sorted[1].val, [9, 10, 11, 12]); // Same as initial
    }
}
