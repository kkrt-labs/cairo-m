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
/// The maximum number of bits for a memory address, set to 30.
/// This limits the memory size to 2^30 elements.
/// TODO: check with Starkware
const MAX_MEMORY_SIZE_BITS: u8 = 30;

/// Custom error types for memory operations.
#[derive(Debug, Clone, Error)]
pub enum MemoryError {
    #[error("Address {addr} is out of bounds. Maximum allowed address is {max_addr}")]
    AddressOutOfBounds { addr: M31, max_addr: u32 },
    #[error("Cannot project value at address {addr} to base field M31: {value:?}")]
    BaseFieldProjectionFailed { addr: M31, value: QM31 },
    #[error("Memory cell at address {addr} is not initialized")]
    UninitializedMemoryCell { addr: M31 },
}

/// Represents the Cairo M VM's memory, a flat, read-write address space.
///
/// Memory is addressable by `M31` field elements and stores `QM31` values.
#[derive(Debug, Clone, Default)]
pub struct Memory {
    /// The index of the vector corresponds to the memory address.
    /// Instructions and data are stored as `QM31` values.
    pub data: Vec<QM31>,
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
        let max_addr = 1 << MAX_MEMORY_SIZE_BITS;
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
            .data
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
                .data
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
    /// Returns [`MemoryError::BaseFieldProjectionFailed`] if the value at the address
    /// cannot be projected to a base field element.
    pub fn get_data(&self, addr: M31) -> Result<M31, MemoryError> {
        let address = addr.0 as usize;
        let value = self.data.get(address).copied().unwrap_or_default();
        if !value.1.is_zero() || !value.0.1.is_zero() {
            return Err(MemoryError::BaseFieldProjectionFailed { addr, value });
        }
        self.trace.borrow_mut().push(MemoryEntry { addr, value });
        Ok(value.0.0)
    }

    /// Inserts a `QM31` value at a specified validated memory address.
    ///
    /// If the address is beyond the current memory size, the memory is
    /// automatically extended and padded with zeros up to the new address.
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
        Self::validate_address(addr)?;
        let address = addr.0 as usize;

        // Resize vector if necessary
        if address >= self.data.len() {
            self.data.resize(address + 1, QM31::zero());
        }
        self.data[address] = value;
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
        let address = addr.0 as usize;
        if address >= self.data.len() {
            self.data.resize(address + 1, QM31::zero());
        }
        self.data[address] = value;
        Ok(())
    }

    /// Inserts a slice of `QM31` values starting from a given address.
    ///
    /// It validates that both the start and end addresses of the slice are
    /// within memory limits. The memory is resized if necessary to accommodate
    /// the new data.
    ///
    /// # Arguments
    ///
    /// * `start_addr` - The `M31` starting address for the slice.
    /// * `values` - The slice of `QM31` values to insert.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::AddressOutOfBounds`] if any address in the range exceeds the maximum allowed size.
    pub fn insert_slice(&mut self, start_addr: M31, values: &[QM31]) -> Result<(), MemoryError> {
        if values.is_empty() {
            return Ok(());
        }

        // Check that the entire slice fits within memory limits
        let start_address = start_addr.0 as usize;
        let slice_len = values.len();
        // Since we already checked for empty slice, slice_len >= 1
        let last_addr = start_addr.0.checked_add((slice_len - 1) as u32).ok_or(
            MemoryError::AddressOutOfBounds {
                addr: start_addr,
                max_addr: 1 << MAX_MEMORY_SIZE_BITS,
            },
        )?;
        Self::validate_address(last_addr.into())?;

        let end_address = last_addr as usize + 1;

        // Resize vector if necessary
        if end_address > self.data.len() {
            self.data.resize(end_address, QM31::zero());
        }

        // Copy the slice into memory
        self.data[start_address..end_address].copy_from_slice(values);
        self.trace
            .borrow_mut()
            .extend(values.iter().enumerate().map(|(i, value)| MemoryEntry {
                addr: start_addr + M31(i as u32),
                value: *value,
            }));
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
        if fp_min_one_addr >= self.data.len() {
            self.data.resize(fp_min_one_addr + 1, QM31::zero());
        }

        self.data[fp_min_two_addr] = QM31::from_m31_array([fp.0, 0, 0, 0].map(Into::into));
        self.data[fp_min_one_addr] = QM31::from_m31_array([final_pc.0, 0, 0, 0].map(Into::into));

        Ok(())
    }

    /// Extends the memory by appending values from an iterator.
    ///
    /// # Arguments
    ///
    /// * `iter` - An iterator yielding `QM31` values.
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = QM31>,
    {
        self.data.extend(iter);
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
}

impl FromIterator<QM31> for Memory {
    fn from_iter<I: IntoIterator<Item = QM31>>(iter: I) -> Self {
        Self {
            data: iter.into_iter().collect(),
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
        assert!(memory.data.is_empty());
        assert_eq!(memory.data.len(), 0);
    }

    #[test]
    fn test_get_instruction() {
        let addr = M31(42);
        // Create a valid store_imm instruction (opcode 5)
        let value = QM31::from_m31_array([9, 123, 0, 0].map(Into::into));
        let mut data = vec![QM31::zero(); 43];
        data[42] = value;

        let memory = Memory {
            data,
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
            data,
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
        assert_eq!(memory.data.len(), 101);
        assert_eq!(memory.data[100], value);
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
        assert_eq!(memory.data.len(), 43);
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
        assert_eq!(memory.data.len(), 43);
        assert_eq!(memory.trace.borrow().len(), 2);
        assert_eq!(memory.trace.borrow()[0], MemoryEntry { addr, value });
        assert_eq!(memory.trace.borrow()[1], MemoryEntry { addr, value });
    }

    #[test]
    fn test_validate_address() {
        assert!(Memory::validate_address(100.into()).is_ok());
        assert!(Memory::validate_address(1_000_000.into()).is_ok());
        assert!(Memory::validate_address(M31::from((1 << MAX_MEMORY_SIZE_BITS) - 1)).is_ok());
        assert!(Memory::validate_address((1 << MAX_MEMORY_SIZE_BITS).into()).is_ok());
    }

    #[test]
    fn test_validate_address_out_of_bounds() {
        let result = Memory::validate_address(M31::from((1 << MAX_MEMORY_SIZE_BITS) + 1));
        assert!(matches!(
            result,
            Err(MemoryError::AddressOutOfBounds { .. })
        ));
    }

    #[test]
    fn test_insert_slice() {
        let mut memory = Memory::default();
        let start_addr = M31(10);

        // Insert valid instructions in QM31 words:
        // QM31[0]: store_imm (opcode 5, size 3): imm=100, dst_off=0
        // QM31[1]: jmp_abs_imm (opcode 12, size 2): target=42
        let values = vec![
            QM31::from_m31_array([5, 100, 0, 0].map(Into::into)), // store_imm
            QM31::from_m31_array([12, 42, 11, 0].map(Into::into)), // jmp_abs_imm
        ];

        memory.insert_slice(start_addr, &values).unwrap();

        // Verify data is stored correctly by checking raw data
        assert_eq!(memory.data[10], values[0]);
        assert_eq!(memory.data[11], values[1]);

        assert_eq!(memory.trace.borrow().len(), 2);
        // Trace entries from `insert_slice`
        for (i, value) in values.iter().enumerate() {
            assert_eq!(
                memory.trace.borrow()[i],
                MemoryEntry {
                    addr: start_addr + M31(i as u32),
                    value: *value
                }
            );
        }
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

        memory.insert_slice(start_addr, &values).unwrap();

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
    fn test_insert_slice_start_addr_out_of_bounds() {
        let mut memory = Memory::default();
        let invalid_addr = M31::from((1 << MAX_MEMORY_SIZE_BITS) + 1);
        let values = vec![QM31::zero()];
        let result = memory.insert_slice(invalid_addr, &values);
        assert!(matches!(
            result,
            Err(MemoryError::AddressOutOfBounds { .. })
        ));
    }

    #[test]
    fn test_insert_slice_end_addr_out_of_bounds() {
        let mut memory = Memory::default();
        let start_addr = M31::from((1 << MAX_MEMORY_SIZE_BITS) - 5);
        let values = vec![QM31::zero(); 10];
        let result = memory.insert_slice(start_addr, &values);
        assert!(matches!(
            result,
            Err(MemoryError::AddressOutOfBounds { .. })
        ));
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
        assert_eq!(memory.data.len(), 3);
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
        assert_eq!(memory.data.len(), 2);
        // Verify data is stored correctly by checking raw data
        assert_eq!(
            memory.data[0],
            QM31::from_m31_array([100, 0, 0, 0].map(Into::into))
        );
        assert_eq!(
            memory.data[1],
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
}
