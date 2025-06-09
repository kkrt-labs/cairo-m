use num_traits::identities::Zero;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;
use thiserror::Error;

/// The maximum number of bits for a memory address, set to 30.
/// This limits the memory size to 2^30 elements.
/// TODO: check with Starkware
const MAX_MEMORY_SIZE_BITS: u8 = 30;

/// Custom error types for memory operations.
#[derive(Debug, Clone, Error)]
pub enum MemoryError {
    #[error("Address {address} is out of bounds. Maximum allowed address is {max_address}")]
    AddressOutOfBounds { address: M31, max_address: u32 },
    #[error("Cannot project value at address {address} to base field M31: {value:?}")]
    BaseFieldProjectionFailed { address: M31, value: QM31 },
}

/// Represents the Cairo M VM's memory, a flat, read-write address space.
///
/// Memory is addressable by `M31` field elements and stores `QM31` values.
#[derive(Debug, Clone, Default)]
pub struct Memory {
    /// The index of the vector corresponds to the memory address.
    /// Instructions and data are stored as `QM31` values.
    pub data: Vec<QM31>,
}

impl Memory {
    /// Checks if a given memory address is within the allowed range (`0` to `2^MAX_MEMORY_SIZE_BITS`).
    ///
    /// # Arguments
    ///
    /// * `addr` - The `M31` address to validate.
    ///
    /// # Errors
    ///
    /// Returns `MemoryError::AddressOutOfBounds` if the address exceeds the maximum allowed size.
    const fn validate_address(addr: M31) -> Result<(), MemoryError> {
        let max_address = 1 << MAX_MEMORY_SIZE_BITS;
        if addr.0 > max_address {
            return Err(MemoryError::AddressOutOfBounds {
                address: addr,
                max_address,
            });
        }
        Ok(())
    }

    /// Retrieves a `QM31` value from the specified memory address.
    ///
    /// This is used to fetch instructions of the program, which are represented as
    /// `QM31` values. If the address is out of bounds, it returns
    /// `QM31::zero()`, simulating that uninitialized memory reads as zero.
    ///
    /// # Arguments
    ///
    /// * `addr` - The `M31` memory address to read from.
    pub fn get_instruction(&self, addr: M31) -> QM31 {
        let address = addr.0 as usize;
        self.data.get(address).copied().unwrap_or_default()
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
    /// Returns `MemoryError::BaseFieldProjectionFailed` if the value at the address
    /// cannot be projected to a base field element.
    pub fn get_data(&self, addr: M31) -> Result<M31, MemoryError> {
        let address = addr.0 as usize;
        let value = self.data.get(address).copied().unwrap_or_else(QM31::zero);
        if !value.1.is_zero() || !value.0 .1.is_zero() {
            return Err(MemoryError::BaseFieldProjectionFailed {
                address: addr,
                value,
            });
        }
        Ok(value.0 .0)
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
    /// Returns `MemoryError::AddressOutOfBounds` if the address exceeds the maximum allowed size.
    pub fn insert(&mut self, addr: M31, value: QM31) -> Result<(), MemoryError> {
        Self::validate_address(addr)?;
        let address = addr.0 as usize;

        // Resize vector if necessary
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
    /// Returns `MemoryError::AddressOutOfBounds` if any address in the range exceeds the maximum allowed size.
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
                address: start_addr,
                max_address: 1 << MAX_MEMORY_SIZE_BITS,
            },
        )?;
        Self::validate_address(M31::from(last_addr))?;

        let end_address = last_addr as usize + 1;

        // Resize vector if necessary
        if end_address > self.data.len() {
            self.data.resize(end_address, QM31::zero());
        }

        // Copy the slice into memory
        self.data[start_address..end_address].copy_from_slice(values);
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
}

impl FromIterator<QM31> for Memory {
    fn from_iter<I: IntoIterator<Item = QM31>>(iter: I) -> Self {
        Self {
            data: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use num_traits::One;

    use super::*;

    #[test]
    fn test_default() {
        let memory = Memory::default();
        assert!(memory.data.is_empty());
        assert_eq!(memory.data.len(), 0);
    }

    #[test]
    fn test_get_instruction() {
        let mut memory = Memory::default();
        let addr = M31::from(42);
        let value = QM31::from_m31_array([123, 0, 0, 0].map(Into::into));

        memory.insert(addr, value).unwrap();

        assert_eq!(memory.get_instruction(addr), value);
    }

    #[test]
    fn test_get_instruction_from_empty_address() {
        let memory = Memory::default();
        let addr = M31::from(10);

        assert_eq!(memory.get_instruction(addr), QM31::zero());
    }

    #[test]
    fn test_get_data() {
        let mut memory = Memory::default();
        let addr = M31::from(42);
        let value = QM31::from_m31_array([123, 0, 0, 0].map(Into::into));

        memory.insert(addr, value).unwrap();

        assert_eq!(memory.get_data(addr).unwrap(), M31::from(123));
    }

    #[test]
    fn test_get_data_from_empty_address() {
        let memory = Memory::default();
        let addr = M31::from(10);

        assert_eq!(memory.get_data(addr).unwrap(), M31::zero());
    }

    #[test]
    fn test_get_data_error_on_projection_failure() {
        let mut memory = Memory::default();
        let addr = M31::from(42);
        let value = QM31::from_m31_array([0, 0, 123, 0].map(Into::into));

        memory.insert(addr, value).unwrap();

        assert!(matches!(
            memory.get_data(addr),
            Err(MemoryError::BaseFieldProjectionFailed { .. })
        ));
    }

    #[test]
    fn test_insert_instruction_auto_resize() {
        let mut memory = Memory::default();
        let addr = M31::from(100);
        let value = QM31::from_m31_array([42, 0, 0, 0].map(Into::into));

        memory.insert(addr, value).unwrap();

        assert_eq!(memory.data.len(), 101);
        assert_eq!(memory.get_instruction(addr), value);
    }

    #[test]
    fn test_validate_address() {
        assert!(Memory::validate_address(M31::from(100)).is_ok());
        assert!(Memory::validate_address(M31::from(1_000_000)).is_ok());
        assert!(Memory::validate_address(M31::from((1 << MAX_MEMORY_SIZE_BITS) - 1)).is_ok());
        assert!(Memory::validate_address(M31::from(1 << MAX_MEMORY_SIZE_BITS)).is_ok());
    }

    #[test]
    fn test_validate_address_error() {
        let result = Memory::validate_address(M31::from((1 << MAX_MEMORY_SIZE_BITS) + 1));
        assert!(matches!(
            result,
            Err(MemoryError::AddressOutOfBounds { .. })
        ));
    }

    #[test]
    fn test_insert_slice() {
        let mut memory = Memory::default();
        let start_addr = M31::from(10);
        let values = vec![
            QM31::from_m31_array([1, 0, 0, 0].map(Into::into)),
            QM31::from_m31_array([2, 0, 0, 0].map(Into::into)),
            QM31::from_m31_array([3, 0, 0, 0].map(Into::into)),
        ];

        memory.insert_slice(start_addr, &values).unwrap();

        assert_eq!(
            memory.get_instruction(M31::from(10)),
            QM31::from_m31_array([1, 0, 0, 0].map(Into::into))
        );
        assert_eq!(
            memory.get_instruction(M31::from(11)),
            QM31::from_m31_array([2, 0, 0, 0].map(Into::into))
        );
        assert_eq!(
            memory.get_instruction(M31::from(12)),
            QM31::from_m31_array([3, 0, 0, 0].map(Into::into))
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
        assert_eq!(memory.get_instruction(M31::from(0)), M31::from(10).into());
        assert_eq!(memory.get_instruction(M31::one()), M31::from(20).into());
        assert_eq!(memory.get_instruction(M31::from(2)), M31::from(30).into());
    }

    #[test]
    fn test_from_iter() {
        let values = vec![
            QM31::from_m31_array([100, 0, 0, 0].map(Into::into)),
            QM31::from_m31_array([200, 0, 0, 0].map(Into::into)),
        ];

        let memory: Memory = values.into_iter().collect();

        assert_eq!(memory.data.len(), 2);
        assert_eq!(memory.get_instruction(M31::from(0)), M31::from(100).into());
        assert_eq!(memory.get_instruction(M31::one()), M31::from(200).into());
    }
}
