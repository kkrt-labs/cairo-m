use std::collections::HashMap;
use std::iter::Peekable;

use cairo_m_common::instruction::{Instruction, INSTRUCTION_MAX_SIZE};
use cairo_m_common::state::MemoryEntry as RunnerMemoryEntry;
use cairo_m_common::State as VmRegisters;
use num_traits::{One, Zero};
use smallvec::SmallVec;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

use crate::adapter::io::VmImportError;
use crate::preprocessed::range_check::range_check_20::LOG_SIZE_RC_20;

/// Maximum clock difference that can be handled in a single range check (2^20 - 1)
pub const RC20_LIMIT: u32 = (1 << LOG_SIZE_RC_20) - 1;

/// Represents a single memory access in the prover's memory model.
///
/// Each memory entry contains:
/// - The memory address being accessed
/// - The QM31 value stored at that address
/// - The clock time when the access occurred
///
/// This is distinct from the runner's memory entry format.
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct MemoryEntry {
    /// The memory address (M31 field element)
    pub address: M31,
    /// The QM31 value stored at this address
    pub value: QM31,
    /// The clock time when this access occurred
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

/// Represents a data memory access with both previous and current state.
///
/// This structure captures the complete state transition for a memory access
/// that is required for the memory lookups (use previous and emit new).
/// Note that the current clock is in the ExecutionBundle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DataAccess {
    /// The memory address being accessed
    pub address: M31,
    /// The clock time of the previous access to this address
    pub prev_clock: M31,
    /// The memory value before this access
    pub prev_value: M31,
    /// The memory value after this access
    pub value: M31,
}

/// A contiguous range inside the global access log corresponding to memory accesses for a single instruction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AccessSpan {
    /// Start index in the global access log
    pub start: u32,
    /// Number of entries in this step
    pub len: u16,
}

/// Represents an instruction memory access.
///
/// Same as DataAccess but since instruction accesses are only reads, prev_value
/// is the same as value (contained in Instruction). Also for instructions, the address
/// is simply the current pc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionAccess {
    /// The complete instruction that was read from memory
    pub instruction: Instruction,
    /// The clock time of the previous access to this instruction address
    pub prev_clock: M31,
}

/// Represents a complete execution step with all associated memory accesses.
///
/// An execution bundle contains:
/// - The current register state (PC, FP)
/// - The current clock time: clock is incremented at each step
/// - The instruction being executed
/// - Up to 3 operand memory accesses
///
/// The execution bundle contains all the necessary data for generating the witnesses
/// for opcodes. A row of the trace is basically a processed ExecutionBundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionBundle {
    /// The VM register state at this execution step
    pub registers: VmRegisters,
    /// The clock time for this execution step
    pub clock: M31,
    /// The instruction memory access
    pub instruction: InstructionAccess,
    /// Span into the global access log for this step's data memory accesses
    pub access_span: AccessSpan,
}

impl Default for ExecutionBundle {
    fn default() -> Self {
        Self {
            registers: VmRegisters::default(),
            clock: M31::zero(),
            instruction: InstructionAccess {
                instruction: Instruction::Ret {},
                prev_clock: M31::zero(),
            },
            access_span: AccessSpan { start: 0, len: 0 },
        }
    }
}

/// Internal structure for tracking memory access arguments during processing.
///
/// This intermediary structure captures the complete state of a memory access,
/// including both the previous and current values and clock times.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct MemoryArg {
    /// The memory address being accessed
    pub address: M31,
    /// The previous QM31 value at this address
    pub prev_val: QM31,
    /// The current QM31 value at this address
    pub value: QM31,
    /// The clock time of the previous access
    pub prev_clock: M31,
    /// The clock time of the current access
    pub clock: M31,
}

/// Manages the complete memory state for proof generation.
///
///
/// ## For which components ?
/// MEMORY COMPONENT: The Memory struct mainly tracks both initial and final memory states. It is used by the
/// memory component for:
/// - the Memory lookup: + multiplicity * [address, clock, value] where the
///   multiplicity can be -1 (for final memory entries), 0 (unused memory entries), 1 (initial entries).
/// - the Merkle lookup:
///      + [4*addr + 0, value0, depth, root]
///      + [4*addr + 1, value1, depth, root]
///      + [4*addr + 2, value2, depth, root]
///      + [4*addr + 3, value3, depth, root]
///
/// Note that the merkle lookup emits the leaves no matter what the multiplicity of the entry is (ie for
/// used cells as much as unused cells).
/// Also note that in reality the memory component also emits the intermediate nodes for the partial tree,
/// this is to be patched (although currently an intermediate node flag is added to separate leaves and intermediate
/// node emissions)
///
/// CLOCK UPDATE COMPONENT: The clock update data is used by the clock_update component to add artificial "reads" when the clock difference
/// is too large. So if a memory access reads/writes in a cell previously accessed at clk_1 with current_clk - clk_1 > RC20_LIMIT,
/// the prover will:
///     - use the memory access at clk_1 produce a new one at clk_1 + RC20_LIMIT,
///     - if necessary, use this last memory access at clk_1 + RC20_LIMIT and produce one at clk_1 + 2*RC20_LIMIT,
///     - and so on until current_clk - (clk_1 + i*RC20_LIMIT) < RC20_LIMIT.
///
///
/// ## Fields
/// - `initial_memory`: Memory state at the start of execution extended with all first writes.
/// - `final_memory`: Memory state at the end of execution (unlike the initial memory this matches the VM final memory)
/// - `clock_update_data`: Intermediate clock updates for large time gaps
///
/// Note that initial and final memory share the same addresses.
///
///
/// ## Memory Representation
/// Each memory entry is keyed by (address, depth) tuple and contains:
/// - Value: The QM31 value stored
/// - Clock: When the access occurred
/// - Multiplicity: can be -1 (for final memory entries), 0 (unused memory entries), 1 (initial entries)
#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub struct Memory {
    /// Initial memory state: addr => (value, clock, multiplicity)
    pub initial_memory: HashMap<M31, (QM31, M31, M31)>,
    /// Final memory state: (addr, depth) => (value, clock, multiplicity)
    pub final_memory: HashMap<M31, (QM31, M31, M31)>,
    /// Clock update data for handling large time gaps: (addr, clock, value)
    pub clock_update_data: Vec<(M31, M31, QM31)>,
}

/// Iterator that converts runner execution traces into prover execution bundles.
///
/// This iterator processes the raw execution trace from the Cairo-M runner and
/// transforms it into the structured format needed by the prover components.
///
/// ## Type Parameters
/// - `T`: Iterator over VM register states
/// - `M`: Iterator over memory access entries
pub struct ExecutionBundleIterator<T, M>
where
    T: Iterator<Item = VmRegisters>,
    M: Iterator<Item = RunnerMemoryEntry>,
{
    /// Iterator over VM register states
    trace_iter: Peekable<T>,
    /// Iterator over memory log
    memory_iter: Peekable<M>,
    /// Memory state tracker
    memory: Memory,
    /// Global instruction memory access log
    data_accesses: Vec<DataAccess>,
    /// Execution clock, incremented at each VM step
    clock: u32,
    /// Final register state (captured when trace ends)
    final_registers: Option<VmRegisters>,
}

impl<T, M> ExecutionBundleIterator<T, M>
where
    T: Iterator<Item = VmRegisters>,
    M: Iterator<Item = RunnerMemoryEntry>,
{
    /// Creates a new execution bundle iterator.
    ///
    /// ## Arguments
    /// * `trace_iter` - Iterator over VM register states
    /// * `memory_iter` - Iterator over memory access entries
    /// * `initial_memory` - Initial memory state as QM31 values
    ///
    /// ## Returns
    /// A new iterator that will produce execution bundles
    pub fn new(trace_iter: T, memory_iter: M, initial_memory: Vec<QM31>) -> Self {
        Self {
            trace_iter: trace_iter.peekable(),
            memory_iter: memory_iter.peekable(),
            memory: Memory::new(initial_memory),
            data_accesses: Vec::new(),
            clock: 1, // Clock 0 is reserved to preloaded values (like the program, inputs, etc.)
            final_registers: None,
        }
    }

    pub fn peek_initial_registers(&mut self) -> Option<&VmRegisters> {
        self.trace_iter.peek()
    }

    pub fn into_memory_and_data_accesses(self) -> (Memory, Vec<DataAccess>) {
        (self.memory, self.data_accesses)
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

        // Process instruction memory access.
        // Step 1: Read one entry for the instruction's first QM31
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

        // Step 2: Parse opcode from first M31 to determine instruction size
        let opcode_m31 = instruction_entry.value.0 .0;
        let opcode_value = opcode_m31.0;

        // Get instruction size from opcode
        let instruction_size_m31 = match Instruction::size_in_m31s_for_opcode(opcode_value) {
            Some(size) => size,
            None => return Some(Err(VmImportError::InvalidOpcode(opcode_value.into()))),
        };

        // Calculate how many QM31 words we need to read
        let instruction_size_qm31 = instruction_size_m31.div_ceil(4);

        // Collect M31 values for the instruction
        let mut instruction_values = SmallVec::<[M31; INSTRUCTION_MAX_SIZE]>::new();

        // Extract M31s from the first QM31
        let first_qm31 = instruction_entry.value;
        let m31_array = first_qm31.to_m31_array();
        instruction_values.extend(m31_array.iter().take(instruction_size_m31).copied());

        // Step 3: Read additional QM31 words if instruction spans multiple QM31s
        if instruction_size_qm31 > 1 {
            let mem_entry = match self.memory_iter.next() {
                Some(entry) => entry,
                None => return Some(Err(VmImportError::UnexpectedEndOfTrace)),
            };

            let entry = MemoryEntry {
                address: mem_entry.addr,
                value: mem_entry.value,
                clock: self.clock.into(),
            };

            // Push to memory
            self.memory.push(entry);

            // Extract the 5th M31 for U32StoreAddFpImm (which has size 5)
            if instruction_size_m31 > 4 {
                instruction_values.push(entry.value.0 .0);
            }
        }

        // Parse the complete instruction
        let instruction = match Instruction::try_from(instruction_values) {
            Ok(inst) => inst,
            Err(e) => return Some(Err(VmImportError::InvalidInstruction(e))),
        };

        // Create InstructionAccess
        let instruction_access = InstructionAccess {
            instruction,
            prev_clock: instruction_arg.prev_clock,
        };

        // Step 4: Process operand memory accesses based on instruction's opcode
        // The number and type of memory accesses depends on the instruction
        let num_operands = instruction.memory_accesses();

        // Track the contiguous span in the global access log for this step
        let start_idx = self.data_accesses.len() as u32;

        for _i in 0..num_operands {
            // Get the data type for this operand based on the instruction's opcode
            // Single M31 value for Felt operands
            let operand_memory = match self.memory_iter.next() {
                Some(entry) => entry,
                None => return Some(Err(VmImportError::UnexpectedEndOfTrace)),
            };

            let mem_entry = MemoryEntry {
                address: operand_memory.addr,
                value: operand_memory.value,
                clock: self.clock.into(),
            };

            let mem_arg = self.memory.push(mem_entry);

            let data_access: DataAccess = DataAccess {
                address: mem_arg.address,
                prev_clock: mem_arg.prev_clock,
                prev_value: mem_arg.prev_val.0 .0,
                value: mem_arg.value.0 .0,
            };

            self.data_accesses.push(data_access);
        }

        let access_span = AccessSpan {
            start: start_idx,
            len: (self.data_accesses.len() as u32 - start_idx) as u16,
        };

        let bundle = ExecutionBundle {
            registers,
            clock: self.clock.into(),
            instruction: instruction_access,
            access_span,
        };

        self.clock += 1;

        Some(Ok(bundle))
    }
}

impl Memory {
    /// Creates a new Memory instance with initial memory values from the VM output.
    ///
    /// The initial memory is populated with the provided QM31 values,
    /// indexed starting from address 0. Each entry is initialized with:
    /// - Clock = 0 (initial state)
    /// - Multiplicity = 0 (unused until first access)
    ///
    /// ## Arguments
    /// * `initial_memory` - Vector of QM31 values representing the initial memory state
    ///
    /// ## Returns
    /// A new Memory instance ready for execution trace processing
    pub fn new(initial_memory: Vec<QM31>) -> Self {
        let initial_memory_hashmap: HashMap<M31, (QM31, M31, M31)> = initial_memory
            .iter()
            .enumerate()
            .map(|(i, value)| (M31::from(i as u32), (*value, M31::zero(), M31::zero())))
            .collect();
        Self {
            initial_memory: initial_memory_hashmap.clone(),
            final_memory: initial_memory_hashmap,
            clock_update_data: Vec::new(),
        }
    }

    /// Updates multiplicities for public address ranges based on their usage patterns
    pub fn update_multiplicities(&mut self, ranges: &cairo_m_common::PublicAddressRanges) {
        // For program and input addresses
        for addr in ranges.program.clone().chain(ranges.input.clone()) {
            let key = M31::from(addr);

            // Set initial memory multiplicity to 0
            if let Some((value, clock, _)) = self.initial_memory.get(&key).cloned() {
                self.initial_memory.insert(key, (value, clock, M31::zero()));
            }

            // If final memory has multiplicity 0, switch it to -1
            if let Some((value, clock, multiplicity)) = self.final_memory.get(&key).cloned() {
                if multiplicity == M31::zero() {
                    self.final_memory.insert(key, (value, clock, -M31::one()));
                }
            }
        }

        // For output addresses
        for addr in ranges.output.clone() {
            let key = M31::from(addr);

            // Set final memory multiplicity to 0
            if let Some((value, clock, _)) = self.final_memory.get(&key).cloned() {
                self.final_memory.insert(key, (value, clock, M31::zero()));
            }

            // If final memory has multiplicity 0 (which it now does), set initial memory multiplicity to 1
            if let Some((initial_value, initial_clock, _)) = self.initial_memory.get(&key).cloned()
            {
                self.initial_memory
                    .insert(key, (initial_value, initial_clock, M31::one()));
            }
        }
    }

    /// Update Memory with the provided MemoryEntry.
    ///
    /// ## Arguments
    /// * `memory_entry` - The new memory access to process
    ///
    /// ## Returns
    /// A MemoryArg containing the complete memory transition information
    fn push(&mut self, memory_entry: MemoryEntry) -> MemoryArg {
        // No matter what update the final memory with the new memory entry
        // The final memory tracks the "previous data" ie the previous clock and the previous value.
        // - if this memory access is a first write, there won't be any previous entry tracked, in that case
        //   previous entry is (current value, 0, -1), note that the -1 is arbitrary and not used.
        // - if this memory access is a write on an already existing cell or a read, the previous entry is
        //   simply the previous content of final_memory at (addr, HEIGHT).
        let prev_memory_entry = self
            .final_memory
            .insert(
                memory_entry.address,
                (memory_entry.value, memory_entry.clock, -M31::one()),
            )
            .unwrap_or_else(|| (memory_entry.value, M31::zero(), -M31::one()));
        let mut prev_clk = prev_memory_entry.1;
        let current_clk = memory_entry.clock;

        // If it's the first time we access the memory cell, we the initial memory with multiplicity 1 at that address.
        // - again if it's a first write, we insert the memory entry value (coming from the memory log) with clock 0
        //   and multiplicity 1.
        // - for other memory accesses, we simply update the multiplicity to 1 since the value and clock were already
        //   set in Memory::new.
        // NOTE: currently the VM memory is a continuous Vec where empty cells are filled with zero. For example in the
        // initial memory, outputs are always 0 (since they are written later and before the registers in the memory layout).
        if prev_clk == M31::zero() {
            if let Some(initial_memory_cell) = self.initial_memory.get_mut(&memory_entry.address) {
                // Update the multiplicity to 1
                initial_memory_cell.2 = M31::one();
            } else {
                let initial_memory_entry = (memory_entry.value, M31::zero(), M31::one());
                self.initial_memory
                    .insert(memory_entry.address, initial_memory_entry);
            }
        };

        // Because of sparse memory cases (output example mentioned), we need to use the initial memory entry updated as above.
        // Indeed when writing the output, memory_entry.value is the output but initial_memory.get(addr) is 0 (filled by VM).
        // The clock update must consume 0 (emitted by the initial memory) and emit 0. The store opcode will be the one consuming
        // 0 and emitting the actual output.
        let initial_memory_entry = self.initial_memory.get(&memory_entry.address);
        // Check for large clock deltas and generate clock update data if needed
        if current_clk.0 > prev_clk.0 {
            let delta = current_clk.0 - prev_clk.0;
            if delta > RC20_LIMIT {
                // Generate clock update entries for this large delta
                let num_steps = delta / RC20_LIMIT;

                for _ in 0..num_steps {
                    self.clock_update_data.push((
                        memory_entry.address,
                        prev_clk,
                        initial_memory_entry.unwrap().0,
                    ));
                    prev_clk += M31::from(RC20_LIMIT);
                }
            }
        }

        MemoryArg {
            address: memory_entry.address,
            prev_val: prev_memory_entry.0,
            value: memory_entry.value,
            prev_clock: prev_clk, // prev_clk is the last step_clock if there are intermediate steps
            clock: current_clk,
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
        // For a new address, the previous value should be the same as the current value
        assert_eq!(result.prev_val, QM31::from_u32_unchecked(1, 2, 3, 4));
        assert_eq!(result.value, QM31::from_u32_unchecked(1, 2, 3, 4));

        // Verify final_memory was updated
        assert_eq!(
            memory.final_memory[&M31::from(100)],
            (
                QM31::from_u32_unchecked(1, 2, 3, 4),
                M31::from(10),
                -M31::one(),
            )
        );
        // initial_memory should now contain the first access with multiplicity 1
        assert_eq!(
            memory.initial_memory[&M31::from(100)],
            (
                QM31::from_u32_unchecked(1, 2, 3, 4),
                M31::zero(),
                M31::one(),
            )
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

        // Verify final_memory was updated
        assert_eq!(
            memory.final_memory[&M31::from(100)],
            (
                QM31::from_u32_unchecked(5, 6, 7, 8),
                M31::from(20),
                -M31::one(),
            )
        );
        // initial_memory should still contain the first access
        assert_eq!(
            memory.initial_memory[&M31::from(100)],
            (
                QM31::from_u32_unchecked(1, 2, 3, 4),
                M31::zero(),
                M31::one(),
            )
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
        assert_eq!(result.prev_val, QM31::from_u32_unchecked(9, 10, 11, 12)); // Should be same value for first access
        assert_eq!(result.value, QM31::from_u32_unchecked(9, 10, 11, 12));

        // Verify final_memory contains both addresses
        assert_eq!(memory.final_memory.len(), 2);
        assert_eq!(
            memory.final_memory[&M31::from(100)],
            (
                QM31::from_u32_unchecked(1, 2, 3, 4),
                M31::from(10),
                -M31::one(),
            )
        );
        assert_eq!(
            memory.final_memory[&M31::from(200)],
            (
                QM31::from_u32_unchecked(9, 10, 11, 12),
                M31::from(30),
                -M31::one(),
            )
        );
        // initial_memory should contain both addresses
        assert_eq!(memory.initial_memory.len(), 2);
        assert_eq!(
            memory.initial_memory[&M31::from(100)],
            (
                QM31::from_u32_unchecked(1, 2, 3, 4),
                M31::zero(),
                M31::one(),
            )
        );
        assert_eq!(
            memory.initial_memory[&M31::from(200)],
            (
                QM31::from_u32_unchecked(9, 10, 11, 12),
                M31::zero(),
                M31::one(),
            )
        );
    }

    #[test]
    fn test_memory_push_multiple_large_clock_deltas() {
        let mut memory = Memory::default();

        // First entry
        let first_entry = MemoryEntry {
            address: M31::from(100),
            value: QM31::from_u32_unchecked(1, 2, 3, 4),
            clock: M31::from(10),
        };
        memory.push(first_entry);

        // Second entry with very large clock delta requiring multiple steps
        let large_delta = 3 * RC20_LIMIT + 500;
        let second_entry = MemoryEntry {
            address: M31::from(100),
            value: QM31::from_u32_unchecked(5, 6, 7, 8),
            clock: M31::from(10 + large_delta),
        };

        memory.push(second_entry);

        // Verify clock update data was generated for 3 steps
        assert_eq!(memory.clock_update_data.len(), 3);

        // Check first update
        let update1 = &memory.clock_update_data[0];
        assert_eq!(update1.1, M31::from(10)); // prev_clk

        // Check second update
        let update2 = &memory.clock_update_data[1];
        assert_eq!(update2.1, M31::from(10 + RC20_LIMIT)); // prev_clk

        // Check third update
        let update3 = &memory.clock_update_data[2];
        assert_eq!(update3.1, M31::from(10 + 2 * RC20_LIMIT)); // prev_clk
    }

    #[test]
    fn test_memory_push_no_clock_update_for_small_delta() {
        let mut memory = Memory::default();

        // First entry
        let first_entry = MemoryEntry {
            address: M31::from(100),
            value: QM31::from_u32_unchecked(1, 2, 3, 4),
            clock: M31::from(10),
        };
        memory.push(first_entry);

        // Second entry with small clock delta
        let small_delta = RC20_LIMIT - 1; // Just under the limit
        let second_entry = MemoryEntry {
            address: M31::from(100),
            value: QM31::from_u32_unchecked(5, 6, 7, 8),
            clock: M31::from(10 + small_delta),
        };

        memory.push(second_entry);

        // Verify no clock update data was generated
        assert!(memory.clock_update_data.is_empty());
    }

    #[test]
    fn test_memory_push_with_preloaded_memory() {
        // Test with some preloaded memory
        let initial_memory = vec![
            QM31::from_u32_unchecked(10, 20, 30, 40),
            QM31::from_u32_unchecked(50, 60, 70, 80),
        ];
        let mut memory = Memory::new(initial_memory);

        // Verify initial state
        assert_eq!(memory.initial_memory.len(), 2);
        assert_eq!(memory.final_memory.len(), 2);
        assert_eq!(
            memory.initial_memory[&M31::from(0)],
            (
                QM31::from_u32_unchecked(10, 20, 30, 40),
                M31::zero(),
                M31::zero(),
            )
        );
        assert_eq!(
            memory.initial_memory[&M31::from(1)],
            (
                QM31::from_u32_unchecked(50, 60, 70, 80),
                M31::zero(),
                M31::zero(),
            )
        );

        // First push to address 0 must match the preloaded value
        let entry = MemoryEntry {
            address: M31::from(0),
            value: QM31::from_u32_unchecked(10, 20, 30, 40), // Must match preloaded value
            clock: M31::from(5),
        };
        let result = memory.push(entry);

        // Verify the push result
        assert_eq!(result.address, M31::from(0));
        assert_eq!(result.prev_clock, M31::from(0));
        assert_eq!(result.clock, M31::from(5));
        assert_eq!(result.prev_val, QM31::from_u32_unchecked(10, 20, 30, 40));
        assert_eq!(result.value, QM31::from_u32_unchecked(10, 20, 30, 40));

        // Initial memory multiplicity is updated to 1 on first access
        assert_eq!(
            memory.initial_memory[&M31::from(0)],
            (
                QM31::from_u32_unchecked(10, 20, 30, 40),
                M31::zero(),
                M31::one(),
            )
        );
        // Verify final_memory was updated
        assert_eq!(
            memory.final_memory[&M31::from(0)],
            (
                QM31::from_u32_unchecked(10, 20, 30, 40),
                M31::from(5),
                -M31::one(),
            )
        );

        // Now push a different value to the same address
        let second_entry = MemoryEntry {
            address: M31::from(0),
            value: QM31::from_u32_unchecked(100, 200, 300, 400),
            clock: M31::from(10),
        };
        let result = memory.push(second_entry);

        // Verify the second push result
        assert_eq!(result.address, M31::from(0));
        assert_eq!(result.prev_clock, M31::from(5));
        assert_eq!(result.clock, M31::from(10));
        assert_eq!(result.prev_val, QM31::from_u32_unchecked(10, 20, 30, 40));
        assert_eq!(result.value, QM31::from_u32_unchecked(100, 200, 300, 400));

        // Verify final_memory was updated again
        assert_eq!(
            memory.final_memory[&M31::from(0)],
            (
                QM31::from_u32_unchecked(100, 200, 300, 400),
                M31::from(10),
                -M31::one(),
            )
        );
    }
}
