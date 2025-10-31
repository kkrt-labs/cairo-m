use std::cell::RefCell;

use cairo_m_common::instruction::{INSTRUCTION_MAX_SIZE, OPCODE_SIZE_TABLE};
use cairo_m_common::state::MemoryEntry;
use num_traits::One;
use num_traits::identities::Zero;
use smallvec::SmallVec;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;
use thiserror::Error;

/// The number of M31 values that make up a single QM31.
const M31S_IN_QM31: usize = 4;

/// The maximum number of bits for a memory address, set to 28.
/// This limits the memory size to 2^28 elements.
const MAX_MEMORY_SIZE_BITS: u8 = 28;

pub const MAX_ADDRESS: usize = (1 << MAX_MEMORY_SIZE_BITS) - 1;

/// Number of bits in a U32 limb (16 bits per limb for 32-bit values)
pub const U32_LIMB_BITS: u32 = 16;

/// Mask for a U32 limb (0xFFFF)
pub const U32_LIMB_MASK: u32 = (1 << U32_LIMB_BITS) - 1;

/// Custom error types for memory operations.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum MemoryError {
    #[error("Address {addr} is out of bounds. Maximum allowed address is {max_addr}")]
    AddressOutOfBounds { addr: M31, max_addr: u32 },
    #[error("Cannot project value at address {addr} to base field M31: {value:?}")]
    BaseFieldProjectionFailed { addr: M31, value: QM31 },
    #[error("Memory cell at address {addr} is not initialized")]
    UninitializedMemoryCell { addr: M31 },
    #[error(
        "U32 source limbs exceed 16-bit range: limb_lo={}, limb_hi={}",
        limb_lo,
        limb_hi
    )]
    U32LimbOutOfRange { limb_lo: u32, limb_hi: u32 },
}

/// Represents the Cairo M VM's memory, a flat, read-write address space.
///
/// Memory is addressable by `M31` field elements and stores `QM31` values.
#[derive(Debug, Clone, Default)]
pub struct Memory {
    /// Memory is split between local values containing the program and stack, and the heap.
    /// From the VM point of view, there is no distinction between the two.
    /// Addresses close to 0 will be appended to the locals vector, and addresses close to ADDRESS_MAX will be appended to the heap vector, with addresses going downwards :
    /// heap[i] maps to ADDRESS_MAX - i
    pub(crate) locals: Vec<QM31>,
    pub(crate) heap: Vec<QM31>,
    /// A trace of memory accesses.
    ///
    /// The trace is wrapped in a `RefCell` to enable interior mutability. This
    /// allows methods with immutable `&self` receivers, like `get_data`, to
    /// modify the trace. This design choice separates the logical immutability
    /// of an operation from the implementation detail of tracing.
    pub trace: RefCell<Vec<MemoryEntry>>,
}

impl Memory {
    /// Checks if a given memory address is within the allowed range (`0` to `1 << MAX_MEMORY_SIZE_BITS`).
    ///
    /// # Arguments
    ///
    /// * `addr` - The `M31` address to validate.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::AddressOutOfBounds`] if the address exceeds the maximum allowed size.
    const fn validate_address(addr: M31) -> Result<(), MemoryError> {
        let max_addr = MAX_ADDRESS as u32;
        if addr.0 > max_addr {
            return Err(MemoryError::AddressOutOfBounds { addr, max_addr });
        }
        Ok(())
    }

    /// Retrieves a complete instruction from memory, handling both single and multi-word instructions.
    ///
    /// This method fetches the first QM31 word to determine the opcode, then fetches
    /// additional QM31 words if needed for multi-word instructions.
    ///
    /// # Arguments
    ///
    /// * `addr` - The `M31` memory address of the instruction's first word.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::UninitializedMemoryCell`] if any required memory cell is not initialized.
    pub fn get_instruction(
        &self,
        addr: M31,
    ) -> Result<SmallVec<[M31; INSTRUCTION_MAX_SIZE]>, MemoryError> {
        // Fetch first QM31 word
        let address = addr.0 as usize;
        let first_qm31 = self
            .locals
            .get(address)
            .copied()
            .ok_or(MemoryError::UninitializedMemoryCell { addr })?;
        let mut trace = self.trace.borrow_mut();
        trace.push(MemoryEntry {
            addr,
            value: first_qm31,
        });

        // Decompose QM31 once and reuse
        let first_qm31_array = first_qm31.to_m31_array();
        let opcode = first_qm31_array[0].0;

        // Determine instruction size using const lookup table
        let size_in_m31s = match OPCODE_SIZE_TABLE
            .get(opcode as usize)
            .and_then(|&size| size)
        {
            Some(size) => size,
            None => {
                // Invalid opcode - return just the first QM31's M31 values
                // The VM will validate and return the proper error
                return Ok(SmallVec::from_slice(&first_qm31_array));
            }
        };

        // Pre-allocate a SmallVec with the first QM31 word.
        // This is the most common path.
        let mut instruction_m31s = SmallVec::from_slice(&first_qm31_array);

        // Calculate how many QM31 words the instruction occupies.
        // For sizes 1-4, this is 1. For size 5, this is 2.
        let size_in_qm31s = size_in_m31s.div_ceil(M31S_IN_QM31);

        // Loop to fetch any additional words.
        // This loop is highly predictable: it runs 0 times for most instructions
        // and 1 time for the single 5-M31 instruction.
        for i in 1..size_in_qm31s {
            let next_addr = addr + M31::from(i as u32);
            let qm31_word = self
                .locals
                .get(next_addr.0 as usize)
                .copied()
                .ok_or(MemoryError::UninitializedMemoryCell { addr: next_addr })?;

            trace.push(MemoryEntry {
                addr: next_addr,
                value: qm31_word,
            });
            instruction_m31s.extend_from_slice(&qm31_word.to_m31_array());
        }

        // Ensure the final vector has the exact size.
        instruction_m31s.truncate(size_in_m31s);

        Ok(instruction_m31s)
    }

    /// Retrieves a `QM31` value from memory without recording a trace entry.
    ///
    /// This method is a helper for the get_data and get_data_no_trace methods.
    ///
    /// # Arguments
    ///
    /// * `addr` - The `M31` memory address to read from.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::AddressOutOfBounds`] if the address exceeds the maximum allowed size.
    /// Returns [`MemoryError::BaseFieldProjectionFailed`] if the value at the address
    /// cannot be projected to a base field element.
    fn get_qm31_no_trace(&self, addr: M31) -> Result<QM31, MemoryError> {
        Self::validate_address(addr)?;
        let address = addr.0 as usize;
        let locals_address = address;
        let heap_address = MAX_ADDRESS - address;
        let value = self
            .locals
            .get(locals_address)
            .copied()
            .or_else(|| self.heap.get(heap_address).copied())
            .unwrap_or_else(QM31::zero);
        if !value.1.is_zero() || !value.0.1.is_zero() {
            return Err(MemoryError::BaseFieldProjectionFailed { addr, value });
        }
        Ok(value)
    }

    /// Retrieves a value from memory and projects it to a base field element `M31`.
    ///
    /// This is used for instruction arguments or other data that are expected to
    /// be simple field elements. Returns an error if the retrieved `QM31` value cannot
    /// be projected to the base field (i.e., its extension components are non-zero).
    ///
    /// # Arguments
    ///
    /// * `addr` - The `M31` memory address to read from.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::AddressOutOfBounds`] if the address exceeds the maximum allowed size.
    /// Returns [`MemoryError::BaseFieldProjectionFailed`] if the value at the address
    /// cannot be projected to a base field element.
    pub fn get_data(&self, addr: M31) -> Result<M31, MemoryError> {
        let value = self.get_qm31_no_trace(addr)?;
        self.trace.borrow_mut().push(MemoryEntry { addr, value });
        Ok(value.0.0)
    }

    /// Retrieves a value from memory and projects it to a base field element `M31` without recording a trace entry.
    ///
    /// This method is used for debugging instructions that should not affect the execution trace.
    ///
    /// # Arguments
    ///
    /// * `addr` - The `M31` memory address to read from.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::AddressOutOfBounds`] if the address exceeds the maximum allowed size.
    /// Returns [`MemoryError::BaseFieldProjectionFailed`] if the value at the address
    /// cannot be projected to a base field element.
    pub fn get_data_no_trace(&self, addr: M31) -> Result<M31, MemoryError> {
        let value = self.get_qm31_no_trace(addr)?;
        Ok(value.0.0)
    }

    /// Inserts a `QM31` value at a specified validated memory address.
    ///
    ///
    /// # Arguments
    ///
    /// * `addr` - The `M31` memory address to write to.
    /// * `value` - The `QM31` value to insert.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::AddressOutOfBounds`] if the address exceeds the maximum allowed size.
    pub fn insert(&mut self, addr: M31, value: QM31) -> Result<(), MemoryError> {
        self.insert_no_trace(addr, value)?;
        self.trace.borrow_mut().push(MemoryEntry { addr, value });
        Ok(())
    }

    /// Inserts a `QM31` value at a specified validated memory address without logging a trace entry.
    ///
    /// # Arguments
    ///
    /// * `addr` - The `M31` memory address to write to.
    /// * `value` - The `QM31` value to insert.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::AddressOutOfBounds`] if the address exceeds the maximum allowed size.
    pub(crate) fn insert_no_trace(&mut self, addr: M31, value: QM31) -> Result<(), MemoryError> {
        Self::validate_address(addr)?;
        let locals_address = addr.0 as usize;
        let heap_address = MAX_ADDRESS - addr.0 as usize;

        if locals_address < self.locals.len() {
            self.locals[locals_address] = value;
            return Ok(());
        }
        if heap_address < self.heap.len() {
            self.heap[heap_address] = value;
            return Ok(());
        }
        // Find nearest vector to resize
        let locals_distance = locals_address - self.locals.len();
        let heap_distance = heap_address - self.heap.len();
        if locals_distance < heap_distance {
            self.locals.resize(locals_address + 1, QM31::zero());
            self.locals[locals_address] = value;
            return Ok(());
        }
        self.heap.resize(heap_address + 1, QM31::zero());
        self.heap[heap_address] = value;

        Ok(())
    }

    /// Initializes the call stack for the program entrypoint execution.
    ///
    /// Sets up the return frame pointer and return address values required by the Cairo M
    /// calling convention to properly terminate program execution. This stores:
    /// - Frame pointer value at address `fp-2`
    /// - Final program counter at address `fp-1`
    ///
    /// ## Arguments
    ///
    /// * `final_pc` - The final program counter where execution should end
    /// * `fp` - The frame pointer for the entrypoint function
    ///
    /// ## Errors
    ///
    /// Returns [`MemoryError::AddressOutOfBounds`] if either `fp-2` or `fp-1`
    /// addresses exceed the maximum allowed memory size.
    ///
    /// ## Note
    ///
    /// This function is only used in the VM `run_from_entrypoint` method.
    /// It deliberately avoids adding entries to the memory trace.
    pub(crate) fn insert_entrypoint_call(
        &mut self,
        final_pc: &M31,
        fp: &M31,
    ) -> Result<(), MemoryError> {
        let fp_min_two = *fp - M31(2);
        let fp_min_one = *fp - M31::one();
        Self::validate_address(fp_min_two)?;
        Self::validate_address(fp_min_one)?;

        let fp_min_two_addr = fp_min_two.0 as usize;
        let fp_min_one_addr = fp_min_one.0 as usize;
        if fp_min_one_addr >= self.locals.len() {
            self.locals.resize(fp_min_one_addr + 1, QM31::zero());
        }

        self.locals[fp_min_two_addr] = QM31::from_m31_array([fp.0, 0, 0, 0].map(Into::into));
        self.locals[fp_min_one_addr] = QM31::from_m31_array([final_pc.0, 0, 0, 0].map(Into::into));

        Ok(())
    }

    /// Extends the memory by appending values from an iterator.
    /// We will assume an empty heap and extend the locals vector.
    ///
    /// # Arguments
    ///
    /// * `iter` - An iterator yielding `QM31` values.
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = QM31>,
    {
        self.locals.extend(iter);
    }

    /// Serializes the trace to a byte vector.
    ///
    /// Each trace entry consists of an `addr` (`M31`) and a `value` (`QM31`).
    /// A `QM31` is composed of 4 `M31` values.
    /// This function serializes the entire trace as a flat sequence of bytes.
    /// For each entry, it serializes `addr` and then the 4 components of `value`
    /// into little-endian bytes.
    ///
    /// The final output is a single `Vec<u8>` concatenating the bytes for all entries.
    ///
    /// ## Returns
    ///
    /// A `Vec<u8>` containing the serialized trace data.
    pub fn serialize_trace(&self) -> Vec<u8> {
        self.trace
            .borrow()
            .iter()
            .flat_map(|entry| {
                [
                    entry.addr.0,
                    entry.value.0.0.0,
                    entry.value.0.1.0,
                    entry.value.1.0.0,
                    entry.value.1.1.0,
                ]
            })
            .flat_map(u32::to_le_bytes)
            .collect()
    }

    /// Read a 32-bit value (little-endian) stored as two 16-bit limbs at `addr`.
    pub fn get_u32(&self, addr: M31) -> Result<u32, MemoryError> {
        let limb_lo = self.get_data(addr)?;
        let limb_hi = self.get_data(addr + M31::one())?;

        if limb_lo.0 > U32_LIMB_MASK || limb_hi.0 > U32_LIMB_MASK {
            return Err(MemoryError::U32LimbOutOfRange {
                limb_lo: limb_lo.0,
                limb_hi: limb_hi.0,
            });
        }

        Ok((limb_hi.0 << U32_LIMB_BITS) | limb_lo.0)
    }

    /// Read a 32-bit value (little-endian) stored as two 16-bit limbs at `addr` without recording a trace entry.
    ///
    /// This method is used for debugging instructions that should not affect the execution trace.
    pub fn get_u32_no_trace(&self, addr: M31) -> Result<u32, MemoryError> {
        let limb_lo = self.get_data_no_trace(addr)?;
        let limb_hi = self.get_data_no_trace(addr + M31::one())?;

        if limb_lo.0 > U32_LIMB_MASK || limb_hi.0 > U32_LIMB_MASK {
            return Err(MemoryError::U32LimbOutOfRange {
                limb_lo: limb_lo.0,
                limb_hi: limb_hi.0,
            });
        }

        Ok((limb_hi.0 << U32_LIMB_BITS) | limb_lo.0)
    }

    /// Write `value` as two 16-bit limbs (little-endian) at `addr`.
    pub fn insert_u32(&mut self, addr: M31, value: u32) -> Result<(), MemoryError> {
        let limb_lo = M31::from(value & U32_LIMB_MASK);
        let limb_hi = M31::from((value >> U32_LIMB_BITS) & U32_LIMB_MASK);

        self.insert(addr, limb_lo.into())?;
        self.insert(addr + M31::one(), limb_hi.into())?;
        Ok(())
    }
}

/// Since this is used to load programs, we dump the iterator into the locals vector.
impl FromIterator<QM31> for Memory {
    fn from_iter<I: IntoIterator<Item = QM31>>(iter: I) -> Self {
        Self {
            locals: iter.into_iter().collect(),
            heap: vec![],
            trace: RefCell::new(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    #[test]
    fn test_default() {
        let memory = Memory::default();
        assert!(memory.locals.is_empty());
        assert_eq!(memory.locals.len(), 0);
        assert!(memory.heap.is_empty());
        assert_eq!(memory.heap.len(), 0);
    }

    #[test]
    fn test_get_instruction() {
        let addr = M31(42);
        // Create a valid store_imm instruction (opcode 5)
        let value = QM31::from_m31_array([9, 123, 0, 0].map(Into::into));
        let mut data = vec![QM31::zero(); 43];
        data[42] = value;

        let memory = Memory {
            locals: data,
            heap: vec![],
            trace: RefCell::new(Vec::new()),
        };

        let instruction_m31s = memory.get_instruction(addr).unwrap();
        assert_eq!(instruction_m31s.as_slice(), &[M31(9), M31(123), M31(0)]);
        assert_eq!(memory.trace.borrow().len(), 1);
        assert_eq!(memory.trace.borrow()[0], MemoryEntry { addr, value });
    }

    #[test]
    fn test_get_instruction_from_empty_address() {
        let memory = Memory::default();
        let addr = M31(10);
        assert!(matches!(
            memory.get_instruction(addr),
            Err(MemoryError::UninitializedMemoryCell { .. })
        ));
        assert!(memory.trace.borrow().is_empty());
    }

    #[test]
    fn test_get_data() {
        let addr = M31(42);
        let value = QM31::from_m31_array([123, 0, 0, 0].map(Into::into));

        let mut data: Vec<QM31> = vec![QM31::zero(); 43];
        data[42] = value;
        let memory = Memory {
            locals: data,
            heap: vec![],
            trace: RefCell::new(Vec::new()),
        };

        assert_eq!(memory.get_data(addr).unwrap(), M31(123));
        assert_eq!(memory.trace.borrow().len(), 1);
        assert_eq!(memory.trace.borrow()[0], MemoryEntry { addr, value });
    }

    #[test]
    fn test_get_data_from_empty_address() {
        let memory = Memory::default();
        let addr = M31(10);
        assert_eq!(memory.get_data(addr).unwrap(), M31::zero());
        assert_eq!(memory.trace.borrow().len(), 1);
        assert_eq!(
            memory.trace.borrow()[0],
            MemoryEntry {
                addr,
                value: QM31::zero()
            }
        );
    }

    #[test]
    fn test_get_data_error_on_projection_failure() {
        let mut memory = Memory::default();
        let addr = M31(42);
        let value = QM31::from_m31_array([0, 0, 123, 0].map(Into::into));
        memory.insert(addr, value).unwrap();
        memory.trace.borrow_mut().clear();
        assert!(matches!(
            memory.get_data(addr),
            Err(MemoryError::BaseFieldProjectionFailed { .. })
        ));
        assert!(memory.trace.borrow().is_empty());
    }

    #[test]
    fn test_insert() {
        let mut memory = Memory::default();
        let addr = M31(100);
        let value = QM31::from_m31_array([42, 0, 0, 0].map(Into::into));
        memory.insert(addr, value).unwrap();
        assert_eq!(memory.locals.len(), 101);
        assert_eq!(memory.locals[100], value);
        assert_eq!(memory.trace.borrow().len(), 1);
        assert_eq!(memory.trace.borrow()[0], MemoryEntry { addr, value });
    }

    #[test]
    fn test_insert_then_get_instruction() {
        let mut memory = Memory::default();
        let addr = M31(42);
        // Create a valid store_imm instruction (opcode 5)
        let value = QM31::from_m31_array([9, 123, 0, 0].map(Into::into));

        memory.insert(addr, value).unwrap();
        let instruction_m31s = memory.get_instruction(addr).unwrap();
        assert_eq!(instruction_m31s.as_slice(), &[M31(9), M31(123), M31(0)]);
        assert_eq!(memory.locals.len(), 43);
        assert_eq!(memory.trace.borrow().len(), 2);
        assert_eq!(memory.trace.borrow()[0], MemoryEntry { addr, value });
        assert_eq!(memory.trace.borrow()[1], MemoryEntry { addr, value });
    }

    #[test]
    fn test_insert_then_get_data() {
        let mut memory = Memory::default();
        let addr = M31(42);
        let value = QM31::from_m31_array([123, 0, 0, 0].map(Into::into));

        memory.insert(addr, value).unwrap();
        assert_eq!(memory.get_data(addr).unwrap(), value.0.0);
        assert_eq!(memory.locals.len(), 43);
        assert_eq!(memory.trace.borrow().len(), 2);
        assert_eq!(memory.trace.borrow()[0], MemoryEntry { addr, value });
        assert_eq!(memory.trace.borrow()[1], MemoryEntry { addr, value });
    }

    #[test]
    fn test_validate_address() {
        assert!(Memory::validate_address(100.into()).is_ok());
        assert!(Memory::validate_address(1_000_000.into()).is_ok());
        assert!(Memory::validate_address(M31::from(MAX_ADDRESS)).is_ok());
        assert!(Memory::validate_address(M31::from(MAX_ADDRESS + 1)).is_err());
    }

    #[test]
    fn test_validate_address_out_of_bounds() {
        let result = Memory::validate_address(M31::from(MAX_ADDRESS + 1));
        assert!(matches!(
            result,
            Err(MemoryError::AddressOutOfBounds { .. })
        ));
    }

    #[test]
    fn test_get_instruction_multi_qm31() {
        let mut memory = Memory::default();
        let start_addr = M31(0);

        // Insert a U32StoreAddFpImm instruction (opcode 19, size 5 M31s = 2 QM31s)
        // Fields: src_off=1, imm_hi=2, imm_lo=3, dst_off=4
        let values = vec![
            QM31::from_m31_array([19, 1, 2, 3].map(Into::into)), // First 4 M31s
            QM31::from_m31_array([4, 0, 0, 0].map(Into::into)),  // Last M31 + padding
        ];

        memory.insert(start_addr, values[0]).unwrap();
        memory.insert(start_addr + M31(1), values[1]).unwrap();

        // Clear trace to test get_instruction operations
        memory.trace.borrow_mut().clear();

        // Get U32StoreAddFpImm instruction (5 M31s, spans 2 QM31s)
        let inst = memory.get_instruction(start_addr).unwrap();
        assert_eq!(inst.as_slice(), &[M31(19), M31(1), M31(2), M31(3), M31(4)]);

        // Verify trace contains both QM31 accesses
        assert_eq!(memory.trace.borrow().len(), 2);
        assert_eq!(
            memory.trace.borrow()[0],
            MemoryEntry {
                addr: start_addr,
                value: values[0]
            }
        );
        assert_eq!(
            memory.trace.borrow()[1],
            MemoryEntry {
                addr: start_addr + M31(1),
                value: values[1]
            }
        );
    }

    #[test]
    fn test_extend() {
        let mut memory = Memory::default();
        let values = vec![
            QM31::from_m31_array([10, 0, 0, 0].map(Into::into)),
            QM31::from_m31_array([20, 0, 0, 0].map(Into::into)),
            QM31::from_m31_array([30, 0, 0, 0].map(Into::into)),
        ];
        memory.extend(values);
        assert_eq!(memory.locals.len(), 3);
        assert_eq!(memory.get_data(0.into()).unwrap(), M31(10));
        assert_eq!(memory.get_data(1.into()).unwrap(), M31(20));
        assert_eq!(memory.get_data(2.into()).unwrap(), M31(30));
        assert_eq!(memory.trace.borrow().len(), 3);
    }

    #[test]
    fn test_from_iter() {
        let values = vec![
            QM31::from_m31_array([100, 0, 0, 0].map(Into::into)),
            QM31::from_m31_array([200, 0, 0, 0].map(Into::into)),
        ];
        let memory: Memory = values.into_iter().collect();
        assert_eq!(memory.locals.len(), 2);
        // Verify data is stored correctly by checking raw data
        assert_eq!(
            memory.locals[0],
            QM31::from_m31_array([100, 0, 0, 0].map(Into::into))
        );
        assert_eq!(
            memory.locals[1],
            QM31::from_m31_array([200, 0, 0, 0].map(Into::into))
        );
    }

    #[test]
    fn test_serialize_trace() {
        let mut memory = Memory::default();
        let addr1 = M31(10);
        let value1 = QM31::from_m31_array([1, 2, 3, 4].map(Into::into));
        let addr2 = M31(20);
        let value2 = QM31::from_m31_array([5, 6, 7, 8].map(Into::into));

        memory.insert(addr1, value1).unwrap();
        memory.insert(addr2, value2).unwrap();

        let serialized_trace = memory.serialize_trace();

        // Entry 1: addr=10, value=[1, 2, 3, 4]
        // Entry 2: addr=20, value=[5, 6, 7, 8]
        let expected_bytes = Vec::from(
            [10, 1, 2, 3, 4, 20, 5, 6, 7, 8]
                .map(u32::to_le_bytes)
                .as_flattened(),
        );
        assert_eq!(serialized_trace, expected_bytes);
    }

    #[test]
    fn test_heap_insert_and_get() {
        let mut memory = Memory::default();
        let heap_addr = M31(MAX_ADDRESS as u32); // Maximum address (heap index 0)
        let heap_value = QM31::from_m31_array([42, 0, 0, 0].map(Into::into));

        // Insert into heap
        memory.insert(heap_addr, heap_value).unwrap();

        // Verify heap vector grows and stores value
        assert_eq!(memory.heap.len(), 1);
        assert_eq!(memory.heap[0], heap_value);
        assert_eq!(memory.locals.len(), 0); // Locals should be empty

        // Verify we can read back the data
        assert_eq!(memory.get_data(heap_addr).unwrap(), M31(42));

        // Verify trace entries
        assert_eq!(memory.trace.borrow().len(), 2); // insert + get
        assert_eq!(
            memory.trace.borrow()[0],
            MemoryEntry {
                addr: heap_addr,
                value: heap_value
            }
        );
        assert_eq!(
            memory.trace.borrow()[1],
            MemoryEntry {
                addr: heap_addr,
                value: heap_value
            }
        );
    }

    #[test]
    fn test_heap_multiple_addresses() {
        let mut memory = Memory::default();

        // Insert at multiple heap addresses (high addresses map to low heap indices)
        let addr1 = M31(MAX_ADDRESS as u32); // heap index 0
        let addr2 = M31(MAX_ADDRESS as u32 - 5); // heap index 5
        let addr3 = M31(MAX_ADDRESS as u32 - 10); // heap index 10

        let value1 = QM31::from_m31_array([1, 0, 0, 0].map(Into::into));
        let value2 = QM31::from_m31_array([2, 0, 0, 0].map(Into::into));
        let value3 = QM31::from_m31_array([3, 0, 0, 0].map(Into::into));

        memory.insert(addr1, value1).unwrap();
        memory.insert(addr2, value2).unwrap();
        memory.insert(addr3, value3).unwrap();

        // Heap should grow to accommodate highest heap index (10)
        assert_eq!(memory.heap.len(), 11); // 0 to 10 inclusive
        assert_eq!(memory.heap[0], value1);
        assert_eq!(memory.heap[5], value2);
        assert_eq!(memory.heap[10], value3);

        // Verify unwritten addresses are zero
        assert_eq!(memory.heap[1], QM31::zero());
        assert_eq!(memory.heap[7], QM31::zero());
    }

    #[test]
    fn test_heap_get_from_empty_address() {
        let memory = Memory::default();
        let heap_addr = M31(MAX_ADDRESS as u32);

        // Getting from uninitialized heap address should return zero
        assert_eq!(memory.get_data(heap_addr).unwrap(), M31::zero());

        // Should create trace entry
        assert_eq!(memory.trace.borrow().len(), 1);
        assert_eq!(
            memory.trace.borrow()[0],
            MemoryEntry {
                addr: heap_addr,
                value: QM31::zero()
            }
        );
    }

    #[test]
    fn test_mixed_locals_and_heap() {
        let mut memory = Memory::default();

        // Insert into locals
        let locals_addr = M31(100);
        let locals_value = QM31::from_m31_array([1, 0, 0, 0].map(Into::into));
        memory.insert(locals_addr, locals_value).unwrap();

        // Insert into heap
        let heap_addr = M31(MAX_ADDRESS as u32);
        let heap_value = QM31::from_m31_array([5, 0, 0, 0].map(Into::into));
        memory.insert(heap_addr, heap_value).unwrap();

        // Verify both vectors are populated correctly
        assert_eq!(memory.locals.len(), 101);
        assert_eq!(memory.locals[100], locals_value);
        assert_eq!(memory.heap.len(), 1);
        assert_eq!(memory.heap[0], heap_value);

        // Verify we can read from both
        assert_eq!(memory.get_data(locals_addr).unwrap(), M31(1));
        assert_eq!(memory.get_data(heap_addr).unwrap(), M31(5));

        // Verify trace has all operations
        assert_eq!(memory.trace.borrow().len(), 4);
    }

    #[test]
    fn test_heap_boundary_address() {
        let mut memory = Memory::default();

        // Test exactly at maximum address (heap index 0)
        let max_addr = M31(MAX_ADDRESS as u32);
        let heap_value = QM31::from_m31_array([123, 0, 0, 0].map(Into::into));

        memory.insert(max_addr, heap_value).unwrap();

        assert_eq!(memory.heap.len(), 1);
        assert_eq!(memory.heap[0], heap_value);
        assert_eq!(memory.locals.len(), 0);

        // Test a lower address that should go to locals (not close to max)
        let locals_addr = M31(1000);
        let locals_value = QM31::from_m31_array([456, 0, 0, 0].map(Into::into));

        memory.insert(locals_addr, locals_value).unwrap();

        assert_eq!(memory.locals.len(), 1001); // 0 to 1000 inclusive
        assert_eq!(memory.locals[1000], locals_value);
        assert_eq!(memory.heap.len(), 1); // Heap unchanged
    }
}
