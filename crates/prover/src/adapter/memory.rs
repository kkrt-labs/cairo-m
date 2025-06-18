use std::collections::HashMap;

use stwo_prover::core::fields::qm31::QM31;

use crate::adapter::instructions::MemoryArg;

#[derive(Copy, Clone, Default, Debug)]
pub struct MemoryEntry {
    pub address: u32,
    pub value: [u32; 4],
}

impl From<crate::adapter::io::MemoryEntry> for MemoryEntry {
    fn from(io_entry: crate::adapter::io::MemoryEntry) -> Self {
        Self {
            address: io_entry.address,
            value: io_entry.value,
        }
    }
}

#[derive(Copy, Clone, Default, Debug)]
pub struct TraceEntry {
    pub pc: u32,
    pub fp: u32,
}

impl From<crate::adapter::io::TraceEntry> for TraceEntry {
    fn from(io_entry: crate::adapter::io::TraceEntry) -> Self {
        Self {
            pc: io_entry.pc,
            fp: io_entry.fp,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct MemoryBoundaries {
    pub initial_memory: Vec<(u32, [u32; 4], u32)>,
    pub final_memory: Vec<(u32, [u32; 4], u32)>,
}

#[derive(Debug, Default)]
pub struct MemoryCache {
    clock_cache: HashMap<u32, u32>,
    initial_memory: HashMap<u32, ([u32; 4], u32)>,
    current_memory: HashMap<u32, ([u32; 4], u32)>,
}

impl MemoryCache {
    pub fn push(&mut self, mem_entry: MemoryEntry, clock: u32) -> MemoryArg {
        let prev_clock = self
            .clock_cache
            .get(&mem_entry.address)
            .copied()
            .unwrap_or_else(|| {
                self.initial_memory
                    .insert(mem_entry.address, (mem_entry.value, clock));
                0
            });
        self.clock_cache.insert(mem_entry.address, clock);
        let old_value = self
            .current_memory
            .insert(mem_entry.address, (mem_entry.value, clock))
            .unwrap_or(([0, 0, 0, 0], clock));

        MemoryArg {
            address: mem_entry.address,
            prev_val: QM31::from_u32_unchecked(
                old_value.0[0],
                old_value.0[1],
                old_value.0[2],
                old_value.0[3],
            ),
            value: QM31::from_u32_unchecked(
                mem_entry.value[0],
                mem_entry.value[1],
                mem_entry.value[2],
                mem_entry.value[3],
            ),
            prev_clock,
            clock,
        }
    }

    pub fn get_memory_boundaries(&self) -> MemoryBoundaries {
        MemoryBoundaries {
            initial_memory: self
                .initial_memory
                .iter()
                .map(|(&address, &(value, clock))| (address, value, clock))
                .collect(),
            final_memory: self
                .current_memory
                .iter()
                .map(|(&address, &(value, _clock))| (address, value, _clock))
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
        let mut cache = MemoryCache::default();
        let mem_entry = MemoryEntry {
            address: 42,
            value: [1, 2, 3, 4],
        };
        let clock = 10;

        let memory_arg = cache.push(mem_entry, clock);

        assert_eq!(memory_arg.address, 42);
        assert_eq!(memory_arg.prev_val, QM31::from_u32_unchecked(0, 0, 0, 0));
        assert_eq!(memory_arg.value, QM31::from_u32_unchecked(1, 2, 3, 4));
        assert_eq!(memory_arg.prev_clock, 0);
        assert_eq!(memory_arg.clock, 10);

        // Check internal state
        assert_eq!(cache.clock_cache.get(&42), Some(&10));
        assert_eq!(cache.initial_memory.get(&42), Some(&([1, 2, 3, 4], 10)));
        assert_eq!(cache.current_memory.get(&42), Some(&([1, 2, 3, 4], 10)));
    }

    #[test]
    fn test_memory_cache_push_second_entry_same_address() {
        let mut cache = MemoryCache::default();
        let first_entry = MemoryEntry {
            address: 42,
            value: [1, 2, 3, 4],
        };
        let second_entry = MemoryEntry {
            address: 42,
            value: [5, 6, 7, 8],
        };

        // Push first entry
        cache.push(first_entry, 10);

        // Push second entry to same address
        let memory_arg = cache.push(second_entry, 20);

        assert_eq!(memory_arg.address, 42);
        assert_eq!(memory_arg.prev_val, QM31::from_u32_unchecked(1, 2, 3, 4));
        assert_eq!(memory_arg.value, QM31::from_u32_unchecked(5, 6, 7, 8));
        assert_eq!(memory_arg.prev_clock, 10);
        assert_eq!(memory_arg.clock, 20);

        // Check internal state
        assert_eq!(cache.clock_cache.get(&42), Some(&20));
        assert_eq!(cache.initial_memory.get(&42), Some(&([1, 2, 3, 4], 10))); // Should remain unchanged
        assert_eq!(cache.current_memory.get(&42), Some(&([5, 6, 7, 8], 20))); // Should be updated
    }

    #[test]
    fn test_memory_cache_push_different_addresses() {
        let mut cache = MemoryCache::default();
        let entry1 = MemoryEntry {
            address: 42,
            value: [1, 2, 3, 4],
        };
        let entry2 = MemoryEntry {
            address: 100,
            value: [5, 6, 7, 8],
        };

        cache.push(entry1, 10);
        let memory_arg = cache.push(entry2, 20);

        assert_eq!(memory_arg.address, 100);
        assert_eq!(memory_arg.prev_val, QM31::from_u32_unchecked(0, 0, 0, 0)); // New address, so old value is zero
        assert_eq!(memory_arg.value, QM31::from_u32_unchecked(5, 6, 7, 8));
        assert_eq!(memory_arg.prev_clock, 0); // New address, so previous clock is 0
        assert_eq!(memory_arg.clock, 20);

        // Check both entries exist
        assert_eq!(cache.clock_cache.len(), 2);
        assert_eq!(cache.initial_memory.len(), 2);
        assert_eq!(cache.current_memory.len(), 2);
    }

    #[test]
    fn test_memory_cache_multiple_updates_same_address() {
        let mut cache = MemoryCache::default();
        let address = 42;

        // First update
        let entry1 = MemoryEntry {
            address,
            value: [1, 2, 3, 4],
        };
        cache.push(entry1, 10);

        // Second update
        let entry2 = MemoryEntry {
            address,
            value: [5, 6, 7, 8],
        };
        cache.push(entry2, 20);

        // Third update
        let entry3 = MemoryEntry {
            address,
            value: [9, 10, 11, 12],
        };
        let memory_arg = cache.push(entry3, 30);

        assert_eq!(memory_arg.prev_val, QM31::from_u32_unchecked(5, 6, 7, 8));
        assert_eq!(memory_arg.value, QM31::from_u32_unchecked(9, 10, 11, 12));
        assert_eq!(memory_arg.prev_clock, 20);
        assert_eq!(memory_arg.clock, 30);

        // Initial memory should still be the first value
        assert_eq!(
            cache.initial_memory.get(&address),
            Some(&([1, 2, 3, 4], 10))
        );
        // Current memory should be the latest value
        assert_eq!(
            cache.current_memory.get(&address),
            Some(&([9, 10, 11, 12], 30))
        );
    }

    #[test]
    fn test_get_memory_boundaries_single_entry() {
        let mut cache = MemoryCache::default();
        let entry = MemoryEntry {
            address: 42,
            value: [1, 2, 3, 4],
        };
        cache.push(entry, 10);

        let boundaries = cache.get_memory_boundaries();

        assert_eq!(boundaries.initial_memory.len(), 1);
        assert_eq!(boundaries.final_memory.len(), 1);
        assert_eq!(boundaries.initial_memory[0].0, 42);
        assert_eq!(boundaries.initial_memory[0].1, [1, 2, 3, 4]);
        assert_eq!(boundaries.final_memory[0].0, 42);
        assert_eq!(boundaries.final_memory[0].1, [1, 2, 3, 4]);
    }

    #[test]
    fn test_get_memory_boundaries_multiple_entries() {
        let mut cache = MemoryCache::default();

        // Add multiple entries to different addresses
        cache.push(
            MemoryEntry {
                address: 42,
                value: [1, 2, 3, 4],
            },
            10,
        );
        cache.push(
            MemoryEntry {
                address: 100,
                value: [5, 6, 7, 8],
            },
            20,
        );
        cache.push(
            MemoryEntry {
                address: 200,
                value: [9, 10, 11, 12],
            },
            30,
        );

        let boundaries = cache.get_memory_boundaries();

        assert_eq!(boundaries.initial_memory.len(), 3);
        assert_eq!(boundaries.final_memory.len(), 3);

        // Sort for consistent comparison (HashMap iteration order is not guaranteed)
        let mut initial_sorted = boundaries.initial_memory;
        let mut final_sorted = boundaries.final_memory;
        initial_sorted.sort_by_key(|entry| entry.0);
        final_sorted.sort_by_key(|entry| entry.0);

        assert_eq!(initial_sorted[0].0, 42);
        assert_eq!(initial_sorted[0].1, [1, 2, 3, 4]);
        assert_eq!(initial_sorted[1].0, 100);
        assert_eq!(initial_sorted[1].1, [5, 6, 7, 8]);
        assert_eq!(initial_sorted[2].0, 200);
        assert_eq!(initial_sorted[2].1, [9, 10, 11, 12]);
    }

    #[test]
    fn test_get_memory_boundaries_with_updates() {
        let mut cache = MemoryCache::default();

        // Add initial entry
        cache.push(
            MemoryEntry {
                address: 42,
                value: [1, 2, 3, 4],
            },
            10,
        );
        // Update same address
        cache.push(
            MemoryEntry {
                address: 42,
                value: [5, 6, 7, 8],
            },
            20,
        );
        // Add different address
        cache.push(
            MemoryEntry {
                address: 100,
                value: [9, 10, 11, 12],
            },
            30,
        );

        let boundaries = cache.get_memory_boundaries();

        assert_eq!(boundaries.initial_memory.len(), 2);
        assert_eq!(boundaries.final_memory.len(), 2);

        // Sort for consistent comparison
        let mut initial_sorted = boundaries.initial_memory;
        let mut final_sorted = boundaries.final_memory;
        initial_sorted.sort_by_key(|entry| entry.0);
        final_sorted.sort_by_key(|entry| entry.0);

        // Initial memory should contain first values
        assert_eq!(initial_sorted[0].0, 42);
        assert_eq!(initial_sorted[0].1, [1, 2, 3, 4]); // Initial value
        assert_eq!(initial_sorted[1].0, 100);
        assert_eq!(initial_sorted[1].1, [9, 10, 11, 12]);

        // Final memory should contain current values
        assert_eq!(final_sorted[0].0, 42);
        assert_eq!(final_sorted[0].1, [5, 6, 7, 8]); // Updated value
        assert_eq!(final_sorted[1].0, 100);
        assert_eq!(final_sorted[1].1, [9, 10, 11, 12]); // Same as initial
    }
}
