use std::collections::HashMap;

use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

use crate::adapter::instructions::MemoryArg;

#[derive(Copy, Clone, Default, Debug)]
pub struct MemoryEntry {
    pub address: u32,
    pub value: [u32; 4],
}

impl From<crate::adapter::io::IoMemoryEntry> for MemoryEntry {
    fn from(io_entry: crate::adapter::io::IoMemoryEntry) -> Self {
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

impl From<crate::adapter::io::IoTraceEntry> for TraceEntry {
    fn from(io_entry: crate::adapter::io::IoTraceEntry) -> Self {
        Self {
            pc: io_entry.pc,
            fp: io_entry.fp,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct MemoryBoundaries {
    pub initial_memory: Vec<(M31, QM31, M31)>,
    pub final_memory: Vec<(M31, QM31, M31)>,
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
                // If the memory cell is uninitialized, mark this address as an initial one with dummy value and clock
                // and return previous clock as zero.
                self.initial_memory
                    .insert(mem_entry.address, Default::default());
                0
            });
        // No matter what (initialized cell or not), update the clock cache at the given address with the current clock.
        self.clock_cache.insert(mem_entry.address, clock);
        // Update the current memory at the given address with the current value and clock.
        // If the memory cell is uninitialized, the old value is the dummy value.
        let old_value = self
            .current_memory
            .insert(mem_entry.address, (mem_entry.value, clock))
            .unwrap_or_default();

        // Like so, components can systematically consume the previous (addr, val, clock) and produce the new (addr, val, clock).
        // And the memory component can produce the initial (addr, default_val, default_clock) and consume the final (addr, val, clock).
        MemoryArg {
            address: mem_entry.address.into(),
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
            prev_clock: prev_clock.into(),
            clock: clock.into(),
        }
    }

    pub fn get_memory_boundaries(&self) -> MemoryBoundaries {
        MemoryBoundaries {
            initial_memory: self
                .initial_memory
                .iter()
                .map(|(&address, &(value, clock))| {
                    (
                        address.into(),
                        QM31::from_u32_unchecked(value[0], value[1], value[2], value[3]),
                        clock.into(),
                    )
                })
                .collect(),
            final_memory: self
                .current_memory
                .iter()
                .map(|(&address, &(value, _clock))| {
                    (
                        address.into(),
                        QM31::from_u32_unchecked(value[0], value[1], value[2], value[3]),
                        _clock.into(),
                    )
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use stwo_prover::core::fields::m31::M31;

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

    #[allow(clippy::cognitive_complexity)]
    #[test]
    fn test_memory_cache_push() {
        let mut cache = MemoryCache::default();

        // First memory entry - testing uninitialized cell behavior
        let first_entry = MemoryEntry {
            address: 100,
            value: [1, 2, 3, 4],
        };
        let first_clock = 10;

        let first_result = cache.push(first_entry, first_clock);

        // Verify the result of the first push
        assert_eq!(first_result.address, M31::from(100));
        assert_eq!(first_result.prev_clock, M31::from(0));
        assert_eq!(first_result.clock, M31::from(10));
        assert_eq!(first_result.prev_val, QM31::from_u32_unchecked(0, 0, 0, 0));
        assert_eq!(first_result.value, QM31::from_u32_unchecked(1, 2, 3, 4));

        // Verify internal state after first push
        assert!(cache.initial_memory.contains_key(&100));
        assert_eq!(cache.initial_memory[&100], ([0, 0, 0, 0], 0)); // Arbitrary dummy pushed to initial memory
        assert_eq!(cache.current_memory[&100], ([1, 2, 3, 4], 10)); // New data pushed to current memory
        assert_eq!(cache.clock_cache[&100], 10); // Clock updated

        // Second memory entry - testing initialized cell behavior (same address)
        let second_entry = MemoryEntry {
            address: 100, // Same address
            value: [5, 6, 7, 8],
        };
        let second_clock = 20;

        let second_result = cache.push(second_entry, second_clock);

        // Verify the result of the second push
        assert_eq!(second_result.address, M31::from(100));
        assert_eq!(second_result.prev_clock, M31::from(10)); // Previous clock from first entry
        assert_eq!(second_result.clock, M31::from(20)); // Current clock
        assert_eq!(second_result.prev_val, QM31::from_u32_unchecked(1, 2, 3, 4));
        assert_eq!(second_result.value, QM31::from_u32_unchecked(5, 6, 7, 8));

        // Verify internal state after second push:
        assert_eq!(cache.initial_memory[&100], ([0, 0, 0, 0], 0)); // Initial memory remains unchanged
        assert_eq!(cache.clock_cache[&100], 20); // Clock cache updated
        assert_eq!(cache.current_memory[&100], ([5, 6, 7, 8], 20)); // Current memory updated

        // Test with a different address to verify independent behavior
        let third_entry = MemoryEntry {
            address: 200, // Different address
            value: [9, 10, 11, 12],
        };
        let third_clock = 30;

        let third_result = cache.push(third_entry, third_clock);

        // Verify the result of the third push
        assert_eq!(third_result.address, M31::from(200));
        assert_eq!(third_result.prev_clock, M31::from(0));
        assert_eq!(third_result.clock, M31::from(30));
        assert_eq!(third_result.prev_val, QM31::from_u32_unchecked(0, 0, 0, 0));
        assert_eq!(third_result.value, QM31::from_u32_unchecked(9, 10, 11, 12));

        // Verify that both addresses are now tracked independently
        assert!(cache.initial_memory.contains_key(&100));
        assert!(cache.initial_memory.contains_key(&200));
        assert_eq!(cache.clock_cache.len(), 2);
        assert_eq!(cache.current_memory.len(), 2);
    }
}
