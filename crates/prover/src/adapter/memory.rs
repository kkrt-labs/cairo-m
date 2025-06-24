use std::collections::HashMap;

use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct MemoryEntry {
    pub address: M31,
    pub value: QM31,
    pub clock: M31,
}

impl From<crate::adapter::io::IoMemoryEntry> for MemoryEntry {
    fn from(io_entry: crate::adapter::io::IoMemoryEntry) -> Self {
        Self {
            address: io_entry.address.into(),
            value: QM31::from_u32_unchecked(
                io_entry.value[0],
                io_entry.value[1],
                io_entry.value[2],
                io_entry.value[3],
            ),
            clock: M31::zero(),
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct MemoryArg {
    pub address: M31,
    pub prev_val: QM31,
    pub value: QM31,
    pub prev_clock: M31,
    pub clock: M31,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Memory {
    pub initial_memory: HashMap<M31, (QM31, M31)>,
    pub final_memory: HashMap<M31, (QM31, M31)>,
}

impl Memory {
    pub fn push(&mut self, memory_entry: MemoryEntry) -> MemoryArg {
        let prev_memory_entry = self
            .final_memory
            .insert(
                memory_entry.address,
                (memory_entry.value, memory_entry.clock),
            )
            .unwrap_or_else(|| {
                // If the address is not in the final memory, it's the first time we see it.
                // We initialize it in the initial memory with clock 0.
                let initial_value = (memory_entry.value, M31::zero());
                self.initial_memory
                    .insert(memory_entry.address, initial_value);
                initial_value
            });

        MemoryArg {
            address: memory_entry.address,
            prev_val: prev_memory_entry.0,
            value: memory_entry.value,
            prev_clock: prev_memory_entry.1,
            clock: memory_entry.clock,
        }
    }
}

#[cfg(test)]
mod tests {
    use stwo_prover::core::fields::m31::M31;
    use stwo_prover::core::fields::qm31::QM31;

    use super::*;

    #[test]
    fn test_memory_push_first_entry() {
        let mut memory = Memory::default();

        // First memory entry - testing uninitialized cell behavior
        let first_entry = MemoryEntry {
            address: M31::from(100),
            value: QM31::from_u32_unchecked(1, 2, 3, 4),
            clock: M31::from(10),
        };

        let result = memory.push(first_entry);

        // Verify the result of the first push
        assert_eq!(result.address, M31::from(100));
        assert_eq!(result.prev_clock, M31::from(0)); // Should be 0 for first access
        assert_eq!(result.clock, M31::from(10));
        assert_eq!(result.prev_val, QM31::from_u32_unchecked(1, 2, 3, 4)); // Should be 0 for first access
        assert_eq!(result.value, QM31::from_u32_unchecked(1, 2, 3, 4));

        // Verify internal state after first push
        assert!(memory.initial_memory.contains_key(&M31::from(100)));
        assert_eq!(
            memory.initial_memory[&M31::from(100)],
            (QM31::from_u32_unchecked(1, 2, 3, 4), M31::from(0))
        );
        assert_eq!(
            memory.final_memory[&M31::from(100)],
            (QM31::from_u32_unchecked(1, 2, 3, 4), M31::from(10))
        );
    }

    #[test]
    fn test_memory_push_same_address() {
        let mut memory = Memory::default();

        // First entry
        let first_entry = MemoryEntry {
            address: M31::from(100),
            value: QM31::from_u32_unchecked(1, 2, 3, 4),
            clock: M31::from(10),
        };
        memory.push(first_entry);

        // Second entry to same address
        let second_entry = MemoryEntry {
            address: M31::from(100),
            value: QM31::from_u32_unchecked(5, 6, 7, 8),
            clock: M31::from(20),
        };

        let result = memory.push(second_entry);

        // Verify the result uses previous values
        assert_eq!(result.address, M31::from(100));
        assert_eq!(result.prev_clock, M31::from(10)); // Previous clock from first entry
        assert_eq!(result.clock, M31::from(20));
        assert_eq!(result.prev_val, QM31::from_u32_unchecked(1, 2, 3, 4)); // Previous value
        assert_eq!(result.value, QM31::from_u32_unchecked(5, 6, 7, 8)); // New value

        // Verify final memory is updated
        assert_eq!(
            memory.final_memory[&M31::from(100)],
            (QM31::from_u32_unchecked(5, 6, 7, 8), M31::from(20))
        );
        // Initial memory should remain unchanged
        assert_eq!(
            memory.initial_memory[&M31::from(100)],
            (QM31::from_u32_unchecked(1, 2, 3, 4), M31::from(0))
        );
    }

    #[test]
    fn test_memory_push_different_addresses() {
        let mut memory = Memory::default();

        // First address
        let first_entry = MemoryEntry {
            address: M31::from(100),
            value: QM31::from_u32_unchecked(1, 2, 3, 4),
            clock: M31::from(10),
        };
        memory.push(first_entry);

        // Different address
        let second_entry = MemoryEntry {
            address: M31::from(200),
            value: QM31::from_u32_unchecked(9, 10, 11, 12),
            clock: M31::from(30),
        };

        let result = memory.push(second_entry);

        // Verify the result for new address
        assert_eq!(result.address, M31::from(200));
        assert_eq!(result.prev_clock, M31::from(0)); // Should be 0 for first access
        assert_eq!(result.clock, M31::from(30));
        assert_eq!(result.prev_val, QM31::from_u32_unchecked(9, 10, 11, 12)); // Should be 0 for first access
        assert_eq!(result.value, QM31::from_u32_unchecked(9, 10, 11, 12));

        // Verify both addresses are tracked independently
        assert!(memory.initial_memory.contains_key(&M31::from(100)));
        assert!(memory.initial_memory.contains_key(&M31::from(200)));
        assert!(memory.final_memory.contains_key(&M31::from(100)));
        assert!(memory.final_memory.contains_key(&M31::from(200)));
        assert_eq!(memory.initial_memory.len(), 2);
        assert_eq!(memory.final_memory.len(), 2);
    }
}
