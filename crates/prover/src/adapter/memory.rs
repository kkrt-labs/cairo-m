use std::collections::HashMap;
use std::iter::Peekable;

use cairo_m_common::opcode::{MemoryAccessType, Opcode};
use cairo_m_common::state::MemoryEntry as RunnerMemoryEntry;
use cairo_m_common::State as VmRegisters;
use num_traits::{One, Zero};
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
    pub multiplicity: M31,
}

impl From<MemoryArg> for InstructionAccess {
    fn from(arg: MemoryArg) -> Self {
        Self {
            prev_clock: arg.prev_clock,
            value: arg.value,
        }
    }
}

impl From<MemoryArg> for DataAccess {
    fn from(arg: MemoryArg) -> Self {
        Self {
            address: arg.address,
            prev_clock: arg.prev_clock,
            prev_value: arg.prev_val.0 .0,
            value: arg.value.0 .0,
            multiplicity: M31::zero(),
        }
    }
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
}

/// Reference to a memory cell location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryCellRef {
    /// Reference to an execution bundle: (opcode, bundle_index, operand_index)
    ExecutionBundle(Opcode, usize, usize),
    /// Reference to initial memory: (index in initial_memory vec)
    InitialMemory(usize),
}

/// Memory cell data stored in initial_memory
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitialMemoryCell {
    pub address: M31,
    pub value: QM31,
    pub multiplicity: M31,
}

// TODO: Memory Value can take a value enum(M31, QM31) instead of QM31 to save space
#[derive(Debug, Default, Clone)]
pub struct Memory {
    /// Maps (address, value) to the reference of where this memory cell is stored
    pub memory_pointers: HashMap<(M31, QM31), MemoryCellRef>,
    /// Initial memory cells (those that are read without being written to, like instructions from the program)
    /// The initial_memory is used in the memory component to emit entries that no opcode emits
    pub initial_memory: Vec<InitialMemoryCell>,
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
    states_by_opcodes: HashMap<Opcode, Vec<ExecutionBundle>>,
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
            states_by_opcodes: HashMap::new(),
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

    pub fn into_parts(
        self,
    ) -> (
        HashMap<Opcode, Vec<ExecutionBundle>>,
        Vec<InitialMemoryCell>,
    ) {
        (self.states_by_opcodes, self.memory.initial_memory)
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

        // Parse opcode first to know how to handle memory
        let opcode = match Opcode::try_from(instruction_memory.value) {
            Ok(op) => op,
            Err(e) => return Some(Err(e.into())),
        };

        // Instructions are always reads
        let instruction_arg: InstructionAccess = self
            .memory
            .read(instruction_memory, &mut self.states_by_opcodes)
            .into();

        // Process operand memory accesses
        let opcode_info = opcode.info();
        let memory_pattern = opcode_info.memory_access_pattern;
        let mut operands: [Option<DataAccess>; 3] = [None, None, None];

        // We'll store the bundle first and get its position
        let bundle_idx = self.states_by_opcodes.entry(opcode).or_default().len();

        // Process each operand according to the opcode's memory access pattern
        for (operand_idx, access_type) in memory_pattern.iter().enumerate() {
            let operand_arg: DataAccess = match access_type {
                MemoryAccessType::Read => {
                    let operand_memory = match self.memory_iter.next() {
                        Some(entry) => entry,
                        None => return Some(Err(VmImportError::EmptyTrace)),
                    };
                    self.memory
                        .read(operand_memory, &mut self.states_by_opcodes)
                        .into()
                }
                MemoryAccessType::Write => {
                    let operand_memory = match self.memory_iter.next() {
                        Some(entry) => entry,
                        None => return Some(Err(VmImportError::EmptyTrace)),
                    };
                    let current_ref =
                        MemoryCellRef::ExecutionBundle(opcode, bundle_idx, operand_idx);
                    self.memory
                        .write(operand_memory, current_ref, &mut self.states_by_opcodes)
                        .into()
                }
                MemoryAccessType::Unused => continue,
            };

            operands[operand_idx] = Some(operand_arg);
        }

        let bundle = ExecutionBundle {
            registers,
            clock: self.clock.into(),
            instruction: instruction_arg,
            operands,
        };

        // Store the bundle in states_by_opcodes
        self.states_by_opcodes
            .entry(opcode)
            .or_default()
            .push(bundle);

        self.clock += 1;

        Some(Ok(bundle))
    }
}

impl Memory {
    /// Read from memory, updating multiplicity
    fn read(
        &mut self,
        memory_entry: RunnerMemoryEntry,
        states_by_opcodes: &mut HashMap<Opcode, Vec<ExecutionBundle>>,
    ) -> MemoryArg {
        let key = (memory_entry.addr, memory_entry.value);

        if let Some(&cell_ref) = self.memory_pointers.get(&key) {
            // If the cell has already been read from
            match cell_ref {
                MemoryCellRef::InitialMemory(idx) => {
                    // This is a read on a memory cell that was never written to by the program
                    // e.g. a read on an instruction
                    let cell = &mut self.initial_memory[idx];
                    cell.multiplicity += M31::one();
                    MemoryArg {
                        address: memory_entry.addr,
                        prev_val: memory_entry.value,
                        value: memory_entry.value,
                        prev_clock: M31::zero(), // write happened before the program started, so clock is 0
                    }
                }
                MemoryCellRef::ExecutionBundle(opcode, bundle_idx, operand_idx) => {
                    // This is a read on a memory cell that was written to by the program
                    // e.g by a store opcode
                    // In that case, find the Bundle that wrote to the cell and increment the multiplicity
                    // of the associated write
                    let opcode_bundles = states_by_opcodes.get_mut(&opcode).unwrap();
                    let bundle = opcode_bundles[bundle_idx];
                    let write_data = &mut bundle.operands[operand_idx].unwrap();
                    write_data.multiplicity += M31::one();
                    MemoryArg {
                        address: memory_entry.addr,
                        prev_val: memory_entry.value,
                        value: memory_entry.value,
                        prev_clock: bundle.clock, // clock of the write being read
                    }
                }
            }
        } else {
            // If the cell was never read from, add it to the initial_memory (again, this could be a first read of an instruction)
            let idx = self.initial_memory.len();
            self.initial_memory.push(InitialMemoryCell {
                address: memory_entry.addr,
                value: memory_entry.value,
                multiplicity: M31::from(1), // accounts for the current read
            });
            self.memory_pointers
                .insert(key, MemoryCellRef::InitialMemory(idx));

            MemoryArg {
                address: memory_entry.addr,
                prev_val: memory_entry.value,
                value: memory_entry.value,
                prev_clock: M31::zero(), // again, write happened before the program started, so clock is 0
            }
        }
    }

    /// Write to memory, creating or overriding the memory cell
    fn write(
        &mut self,
        memory_entry: RunnerMemoryEntry,
        current_ref: MemoryCellRef,
        states_by_opcodes: &mut HashMap<Opcode, Vec<ExecutionBundle>>,
    ) -> MemoryArg {
        let key = (memory_entry.addr, memory_entry.value);

        let (prev_val, prev_clock) = if let Some(&cell_ref) = self.memory_pointers.get(&key) {
            match cell_ref {
                MemoryCellRef::InitialMemory(idx) => {
                    // This corresponds to a write to a pre-loaded memory cell (could be an instruction)
                    let cell = &self.initial_memory[idx];

                    // One shouldn't overwrite instructions (as for now)
                    assert_eq!(
                        cell.value.0 .1 * cell.value.1 .0 * cell.value.1 .1,
                        M31::zero(),
                        "Instruction should not be overwritten"
                    );

                    (cell.value.0 .0, M31::zero()) // preloaded values are by definition written at clock 0 (before the program started)
                }
                MemoryCellRef::ExecutionBundle(opcode, bundle_idx, operand_idx) => {
                    // This is corresponds to a write to a memory cell that was already written to by the program
                    // e.g by a store opcode
                    let opcode_bundles = states_by_opcodes.get_mut(&opcode).unwrap();
                    let bundle = opcode_bundles[bundle_idx];
                    let data_access = bundle.operands[operand_idx].unwrap();
                    (data_access.value, bundle.clock) // previously written value and previous clock of the write
                }
            }
        } else {
            // This is a regular write from an opcode so no need to add it to the initial_memory
            // Update memory pointers
            (memory_entry.value.0 .0, M31::zero())
        };
        // Overwrite the memory pointer to point to the current write
        self.memory_pointers.insert(key, current_ref);

        MemoryArg {
            address: memory_entry.addr,
            prev_val: prev_val.into(),
            value: memory_entry.value,
            prev_clock,
        }
    }
}
