use std::cell::RefCell;

use cairo_m_common::instruction::{INSTRUCTION_MAX_SIZE, OPCODE_SIZE_TABLE};
use cairo_m_common::state::MemoryEntry;
use num_traits::identities::Zero;
use num_traits::One;
use smallvec::SmallVec;
use stwo_prover::core::fields::m31::M31;
use thiserror::Error;

/// The maximum number of bits for a memory address, set to 30.
/// This limits the memory size to 2^30 elements.
// TODO: check with Starkware
const MAX_MEMORY_SIZE_BITS: u8 = 30;

/// Number of bits in a U32 limb (16 bits per limb for 32-bit values)
pub const U32_LIMB_BITS: u32 = 16;

/// Mask for a U32 limb (0xFFFF)
pub const U32_LIMB_MASK: u32 = (1 << U32_LIMB_BITS) - 1;

/// Custom error types for memory operations.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum MemoryError {
    #[error("Address {addr} is out of bounds. Maximum allowed address is {max_addr}")]
    AddressOutOfBounds { addr: M31, max_addr: u32 },
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
    /// The index of the vector corresponds to the memory address.
    /// Instructions and data are stored as `M31` values.
    pub data: Vec<M31>,
    /// A trace of memory accesses.
    ///
    /// The trace is wrapped in a `RefCell` to enable interior mutability. This
    /// allows methods with immutable `&self` receivers, like `get_felt`, to
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
    /// This method fetches the first M31 word to determine the opcode, then fetches
    /// additional M31 words if needed for multi-word instructions.
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
        // Fetch first M31 word
        let address = addr.0 as usize;
        let opcode_id = self
            .data
            .get(address)
            .copied()
            .ok_or(MemoryError::UninitializedMemoryCell { addr })?;
        let mut trace = self.trace.borrow_mut();
        trace.push(MemoryEntry {
            addr,
            value: opcode_id,
        });

        // Determine instruction size using const lookup table
        let instruction_size = match OPCODE_SIZE_TABLE
            .get(opcode_id.0 as usize)
            .and_then(|&size| size)
        {
            Some(size) => size,
            None => {
                // Invalid opcode - return just the first QM31's M31 values
                // The VM will validate and return the proper error
                return Ok(SmallVec::from_elem(opcode_id, 1));
            }
        };

        // Pre-allocate a SmallVec with the first M31 word.
        // This is the most common path.
        let mut instruction_m31s = SmallVec::from_elem(opcode_id, 1);

        // Loop to fetch any additional words.
        // This loop is highly predictable: it runs 0 times for most instructions
        // and 1 time for the single 5-M31 instruction.
        for i in 1..instruction_size {
            let next_addr = addr + M31::from(i as u32);
            let m31_word = self
                .data
                .get(next_addr.0 as usize)
                .copied()
                .ok_or(MemoryError::UninitializedMemoryCell { addr: next_addr })?;

            trace.push(MemoryEntry {
                addr: next_addr,
                value: m31_word,
            });
            instruction_m31s.extend([m31_word]);
        }

        // Ensure the final vector has the exact size.
        instruction_m31s.truncate(instruction_size);

        Ok(instruction_m31s)
    }

    /// Retrieves a value from memory.
    ///
    /// # Arguments
    ///
    /// * `addr` - The `M31` memory address to read from.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::BaseFieldProjectionFailed`] if the value at the address
    /// cannot be projected to a base field element.
    pub fn get_felt(&self, addr: M31) -> Result<M31, MemoryError> {
        let address = addr.0 as usize;
        let value = self.data.get(address).copied().unwrap_or_default();
        self.trace.borrow_mut().push(MemoryEntry { addr, value });
        Ok(value)
    }

    /// Inserts a `M31` value at a specified validated memory address.
    ///
    /// If the address is beyond the current memory size, the memory is
    /// automatically extended and padded with zeros up to the new address.
    ///
    /// # Arguments
    ///
    /// * `addr` - The `M31` memory address to write to.
    /// * `value` - The `M31` value to insert.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::AddressOutOfBounds`] if the address exceeds the maximum allowed size.
    pub fn insert(&mut self, addr: M31, value: M31) -> Result<(), MemoryError> {
        Self::validate_address(addr)?;
        let address = addr.0 as usize;

        // Resize vector if necessary
        if address >= self.data.len() {
            self.data.resize(address + 1, M31::zero());
        }
        self.data[address] = value;
        self.trace.borrow_mut().push(MemoryEntry { addr, value });
        Ok(())
    }

    /// Inserts a `M31` value at a specified validated memory address without logging a trace entry.
    ///
    /// # Arguments
    ///
    /// * `addr` - The `M31` memory address to write to.
    /// * `value` - The `M31` value to insert.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::AddressOutOfBounds`] if the address exceeds the maximum allowed size.
    pub(crate) fn insert_no_trace(&mut self, addr: M31, value: M31) -> Result<(), MemoryError> {
        Self::validate_address(addr)?;
        let address = addr.0 as usize;
        if address >= self.data.len() {
            self.data.resize(address + 1, M31::zero());
        }
        self.data[address] = value;
        Ok(())
    }

    /// Inserts a slice of `M31` values starting from a given address.
    ///
    /// It validates that both the start and end addresses of the slice are
    /// within memory limits. The memory is resized if necessary to accommodate
    /// the new data.
    ///
    /// # Arguments
    ///
    /// * `start_addr` - The `M31` starting address for the slice.
    /// * `values` - The slice of `M31` values to insert.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::AddressOutOfBounds`] if any address in the range exceeds the maximum allowed size.
    pub fn insert_slice(&mut self, start_addr: M31, values: &[M31]) -> Result<(), MemoryError> {
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
            self.data.resize(end_address, M31::zero());
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
            self.data.resize(fp_min_one_addr + 1, M31::zero());
        }

        self.data[fp_min_two_addr] = M31::from(fp.0);
        self.data[fp_min_one_addr] = M31::from(final_pc.0);

        Ok(())
    }

    /// Extends the memory by appending values from an iterator.
    ///
    /// # Arguments
    ///
    /// * `iter` - An iterator yielding `M31` values.
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = M31>,
    {
        self.data.extend(iter);
    }

    /// Serializes the trace to a byte vector.
    ///
    /// Each trace entry consists of an `addr` (`M31`) and a `value` (`M31`).
    ///
    /// This function serializes the entire trace as a flat sequence of bytes.
    /// For each entry, it serializes `addr` and then the `value`
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
            .flat_map(|entry| [entry.addr.0, entry.value.0])
            .flat_map(u32::to_le_bytes)
            .collect()
    }

    /// Read a 32-bit value (little-endian) stored as two 16-bit limbs at `addr`.
    pub fn get_u32(&self, addr: M31) -> Result<u32, MemoryError> {
        let limb_lo = self.get_felt(addr)?;
        let limb_hi = self.get_felt(addr + M31::one())?;

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

        self.insert(addr, limb_lo)?;
        self.insert(addr + M31::one(), limb_hi)?;
        Ok(())
    }
}

impl FromIterator<M31> for Memory {
    fn from_iter<I: IntoIterator<Item = M31>>(iter: I) -> Self {
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
        // Create a valid store_imm instruction (opcode 9)
        let values = [M31::from(9), M31::from(123), M31::from(3)];
        let mut data = vec![M31::zero(); 42];
        data.extend(values);

        let memory = Memory {
            data,
            trace: RefCell::new(Vec::new()),
        };

        let instruction_m31s = memory.get_instruction(addr).unwrap();
        assert_eq!(instruction_m31s.as_slice(), &values);
        assert_eq!(memory.trace.borrow().len(), values.len());
        values.iter().enumerate().for_each(|(i, value)| {
            let addr = addr + M31::from(i);
            let value = *value;
            assert_eq!(memory.trace.borrow()[i], MemoryEntry { addr, value });
        });
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
    fn test_get_felt() {
        let addr = M31(42);
        let value = M31::from(123);

        let mut data: Vec<M31> = vec![M31::zero(); 43];
        data[42] = value;
        let memory = Memory {
            data,
            trace: RefCell::new(Vec::new()),
        };

        assert_eq!(memory.get_felt(addr).unwrap(), M31(123));
        assert_eq!(memory.trace.borrow().len(), 1);
        assert_eq!(memory.trace.borrow()[0], MemoryEntry { addr, value });
    }

    #[test]
    fn test_get_felt_from_empty_address() {
        let memory = Memory::default();
        let addr = M31(10);
        assert_eq!(memory.get_felt(addr).unwrap(), M31::zero());
        assert_eq!(memory.trace.borrow().len(), 1);
        assert_eq!(
            memory.trace.borrow()[0],
            MemoryEntry {
                addr,
                value: M31::zero()
            }
        );
    }

    #[test]
    fn test_insert() {
        let mut memory = Memory::default();
        let addr = M31::from(100);
        let value = M31::from(42);
        memory.insert(addr, value).unwrap();
        assert_eq!(memory.data.len(), 101);
        assert_eq!(memory.data[100], value);
        assert_eq!(memory.trace.borrow().len(), 1);
        assert_eq!(memory.trace.borrow()[0], MemoryEntry { addr, value });
    }

    #[test]
    fn test_insert_then_get_instruction() {
        let mut memory = Memory::default();
        let addr = M31(0);
        // Create a valid store_imm instruction (opcode 9)
        let values = [M31::from(9), M31::from(123), M31::from(3)];

        memory.insert_slice(addr, &values).unwrap();

        let instruction_m31s = memory.get_instruction(addr).unwrap();
        assert_eq!(instruction_m31s.as_slice(), &[M31(9), M31(123), M31(3)]);
        assert_eq!(memory.data.len(), 3);
        assert_eq!(memory.trace.borrow().len(), 3 * 2);
        values.iter().enumerate().for_each(|(i, value)| {
            let addr = addr + M31::from(i);
            let value = *value;
            assert_eq!(memory.trace.borrow()[i], MemoryEntry { addr, value });
        });
    }

    #[test]
    fn test_insert_then_get_felt() {
        let mut memory = Memory::default();
        let addr = M31(42);
        let value = M31::from(123);

        memory.insert(addr, value).unwrap();
        assert_eq!(memory.get_felt(addr).unwrap(), value);
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

        // Insert valid instructions in M31 words:
        // store_imm (opcode 9, size 3): imm=100, dst_off=0
        let values = [M31::from(9), M31::from(100), M31::zero()];

        memory.insert_slice(start_addr, &values).unwrap();

        // Verify data is stored correctly by checking raw data
        assert_eq!(memory.data[10], values[0]);
        assert_eq!(memory.data[11], values[1]);
        assert_eq!(memory.data[12], values[2]);

        assert_eq!(memory.trace.borrow().len(), 3);
        // Trace entries from `insert_slice`
        for (i, value) in values.iter().enumerate() {
            assert_eq!(
                memory.trace.borrow()[i],
                MemoryEntry {
                    addr: start_addr + M31::from(i),
                    value: *value
                }
            );
        }
    }

    #[test]
    fn test_get_instruction_multi_m31() {
        let mut memory = Memory::default();
        let start_addr = M31(0);

        // Insert a U32StoreAddFpImm instruction (opcode 19, size 5 M31s = 2 QM31s)
        // Fields: src_off=1, imm_hi=2, imm_lo=3, dst_off=4
        let values = [
            M31::from(19),
            M31::one(),
            M31::from(2),
            M31::from(3),
            M31::from(4),
        ];

        memory.insert_slice(start_addr, &values).unwrap();

        // Clear trace to test get_instruction operations
        memory.trace.borrow_mut().clear();

        // Get U32StoreAddFpImm instruction (5 M31s, spans 2 QM31s)
        let inst = memory.get_instruction(start_addr).unwrap();
        assert_eq!(inst.as_slice(), &values);

        // Verify trace contains both M31 accesses
        assert_eq!(memory.trace.borrow().len(), 5);
        values.iter().enumerate().for_each(|(i, value)| {
            let addr = start_addr + M31::from(i);
            let value = *value;
            assert_eq!(memory.trace.borrow()[i], MemoryEntry { addr, value });
        });
    }

    #[test]
    fn test_insert_slice_start_addr_out_of_bounds() {
        let mut memory = Memory::default();
        let invalid_addr = M31::from((1 << MAX_MEMORY_SIZE_BITS) + 1);
        let values = [M31::zero()];
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
        let values = vec![M31::zero(); 10];
        let result = memory.insert_slice(start_addr, &values);
        assert!(matches!(
            result,
            Err(MemoryError::AddressOutOfBounds { .. })
        ));
    }

    #[test]
    fn test_extend() {
        let mut memory = Memory::default();
        let values = vec![M31::from(10), M31::from(20), M31::from(30)];
        memory.extend(values);
        assert_eq!(memory.data.len(), 3);
        assert_eq!(memory.get_felt(0.into()).unwrap(), M31(10));
        assert_eq!(memory.get_felt(1.into()).unwrap(), M31(20));
        assert_eq!(memory.get_felt(2.into()).unwrap(), M31(30));
        assert_eq!(memory.trace.borrow().len(), 3);
    }

    #[test]
    fn test_from_iter() {
        let values = vec![M31::from(100), M31::from(200)];
        let memory: Memory = values.into_iter().collect();
        assert_eq!(memory.data.len(), 2);
        // Verify data is stored correctly by checking raw data
        assert_eq!(memory.data[0], M31::from(100));
        assert_eq!(memory.data[1], M31::from(200));
    }

    #[test]
    fn test_serialize_trace() {
        let mut memory = Memory::default();
        let addr1 = M31(10);
        let value1 = M31::one();
        let addr2 = M31(20);
        let value2 = M31::from(5);

        memory.insert(addr1, value1).unwrap();
        memory.insert(addr2, value2).unwrap();

        let serialized_trace = memory.serialize_trace();

        // Entry 1: addr=10, value=[1, 2, 3, 4]
        // Entry 2: addr=20, value=[5, 6, 7, 8]
        let expected_bytes = Vec::from([10, 1, 20, 5].map(u32::to_le_bytes).as_flattened());
        assert_eq!(serialized_trace, expected_bytes);
    }
}
