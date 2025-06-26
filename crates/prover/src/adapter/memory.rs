use std::collections::HashMap;
use std::iter::Peekable;

use cairo_m_common::opcode::Opcode;
use cairo_m_common::state::MemoryEntry as RunnerMemoryEntry;
use cairo_m_common::State as VmRegisters;
use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

use crate::adapter::io::VmImportError;

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DataAccess {
    pub address: M31,
    pub prev_clock: M31,
    pub prev_value: M31,
    pub value: M31,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InstructionAccess {
    pub prev_clock: M31,
    pub value: QM31,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExecutionBundle {
    pub registers: VmRegisters,
    pub clock: M31,
    pub instruction: InstructionAccess,
    pub operands: [Option<DataAccess>; 3],
}

/// Intermediary struct to iterate over the VM memory output and construct the ExecutionBundle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct MemoryArg {
    pub address: M31,
    pub prev_val: QM31,
    pub value: QM31,
    pub prev_clock: M31,
    pub clock: M31,
}

// TODO: Memory Value can take a value enum(M31, QM31) instead of QM31 to save space
#[derive(Debug, Default, Eq, PartialEq)]
pub struct Memory {
    pub initial_memory: HashMap<M31, (QM31, M31)>,
    pub final_memory: HashMap<M31, (QM31, M31)>,
}

pub struct ExecutionBundleIterator<T, M>
where
    T: Iterator<Item = VmRegisters>,
    M: Iterator<Item = RunnerMemoryEntry>,
{
    trace_iter: Peekable<T>,
    memory_iter: Peekable<M>,
    memory: Memory,
    clock: u32,
    final_registers: Option<VmRegisters>,
}

impl<T, M> ExecutionBundleIterator<T, M>
where
    T: Iterator<Item = VmRegisters>,
    M: Iterator<Item = RunnerMemoryEntry>,
{
    pub fn new(trace_iter: T, memory_iter: M) -> Self {
        Self {
            trace_iter: trace_iter.peekable(),
            memory_iter: memory_iter.peekable(),
            memory: Memory::default(),
            clock: 1, // Initial memory uses clock = 0
            final_registers: None,
        }
    }

    pub fn peek_initial_registers(&mut self) -> Option<&VmRegisters> {
        self.trace_iter.peek()
    }

    pub fn into_memory(self) -> Memory {
        self.memory
    }

    pub const fn get_final_registers(&self) -> Option<VmRegisters> {
        self.final_registers
    }
}

impl<T, M> Iterator for ExecutionBundleIterator<T, M>
where
    T: Iterator<Item = VmRegisters>,
    M: Iterator<Item = RunnerMemoryEntry>,
{
    type Item = Result<ExecutionBundle, VmImportError>;

    fn next(&mut self) -> Option<Self::Item> {
        let registers = self.trace_iter.next()?;

        // Check if this is the last entry
        if self.trace_iter.peek().is_none() {
            // This is the final state - store it and return None
            self.final_registers = Some(registers);
            return None;
        }

        // Process instruction memory access
        let instruction_memory = match self.memory_iter.next() {
            Some(entry) => entry,
            None => return Some(Err(VmImportError::EmptyTrace)),
        };

        let instruction_entry = MemoryEntry {
            address: instruction_memory.addr,
            value: instruction_memory.value,
            clock: self.clock.into(),
        };

        let instruction_arg = self.memory.push(instruction_entry);

        // Parse opcode
        let opcode = match Opcode::try_from(instruction_entry.value) {
            Ok(op) => op,
            Err(e) => return Some(Err(e.into())),
        };

        // Create InstructionAccess
        let instruction = InstructionAccess {
            prev_clock: instruction_arg.prev_clock,
            value: instruction_arg.value,
        };

        // Process operand memory accesses
        let num_operands = opcode.info().memory_accesses;
        let mut operands: [Option<DataAccess>; 3] = [None, None, None];

        for operand_slot in operands.iter_mut().take(num_operands) {
            let operand_memory = match self.memory_iter.next() {
                Some(entry) => entry,
                None => return Some(Err(VmImportError::EmptyTrace)),
            };

            let operand_entry = MemoryEntry {
                address: operand_memory.addr,
                value: operand_memory.value,
                clock: self.clock.into(),
            };

            let operand_arg = self.memory.push(operand_entry);

            let data_access = DataAccess {
                address: operand_arg.address,
                prev_clock: operand_arg.prev_clock,
                prev_value: operand_arg.prev_val.0 .0,
                value: operand_arg.value.0 .0,
            };

            *operand_slot = Some(data_access);
        }

        let bundle = ExecutionBundle {
            registers,
            clock: self.clock.into(),
            instruction,
            operands,
        };

        self.clock += 1;

        Some(Ok(bundle))
    }
}

impl Memory {
    fn push(&mut self, memory_entry: MemoryEntry) -> MemoryArg {
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
