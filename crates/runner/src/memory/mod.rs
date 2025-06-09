use num_traits::identities::Zero;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

/// The maximum number of bits for a memory address, set to 30.
/// This limits the memory size to 2^30 elements.
const MAX_MEMORY_SIZE_BITS: u8 = 30;

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
        self.data.get(address).copied().unwrap_or_else(QM31::zero)
    }

    /// Retrieves a value from memory and projects it to a base field element `M31`.
    ///
    /// This is used for instruction arguments or other data that are expected to
    /// be simple field elements. It asserts that the retrieved `QM31` value is
    /// in the base field (i.e., its extension components are zero).
    ///
    /// # Arguments
    ///
    /// * `addr` - The `M31` memory address to read from.
    ///
    /// # Panics
    ///
    /// Panics if the value at the address is not a base field element.
    pub fn get_data(&self, addr: M31) -> M31 {
        let qm31_value = self.get_instruction(addr);
        assert!(qm31_value.1.is_zero());
        assert!(qm31_value.0 .1.is_zero());
        qm31_value.0 .0
    }

    /// Checks if a given memory address is within the allowed range (`0` to `2^MAX_MEMORY_SIZE_BITS`).
    ///
    /// # Arguments
    ///
    /// * `address` - The `M31` address to validate.
    ///
    /// # Panics
    ///
    /// Panics if the address is out of bounds. This helps prevent memory
    /// access violations.
    fn validate_address(address: M31) {
        if address.0 > (1 << MAX_MEMORY_SIZE_BITS) {
            panic!("Max memory size is 2 ** {MAX_MEMORY_SIZE_BITS}; got address: {address}.")
        }
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
    pub fn insert(&mut self, addr: M31, value: QM31) {
        Self::validate_address(addr);
        let address = addr.0 as usize;

        // Resize vector if necessary
        if address >= self.data.len() {
            self.data.resize(address + 1, QM31::zero());
        }

        self.data[address] = value;
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
    pub fn insert_slice(&mut self, start_addr: M31, values: &[QM31]) {
        if values.is_empty() {
            return;
        }

        // Check that the entire slice fits within memory limits
        let start_address = start_addr.0 as usize;
        let slice_len = values.len();
        // Since we already checked for empty slice, slice_len >= 1
        let last_addr = start_addr.0.saturating_add((slice_len - 1) as u32);
        Self::validate_address(M31::from(last_addr));

        let end_address = start_address + slice_len;

        // Resize vector if necessary
        if end_address > self.data.len() {
            self.data.resize(end_address, QM31::zero());
        }

        // Copy the slice into memory
        self.data[start_address..end_address].copy_from_slice(values);
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
        // Create a QM31 value that represents a base field element (only first component non-zero)
        let value = QM31::from_m31(M31::from(123), M31::from(123), M31::zero(), M31::zero());

        memory.insert(addr, value);

        let retrieved = memory.get_instruction(addr);
        assert_eq!(retrieved, value); // Should get the full QM31 value
    }

    #[test]
    fn test_get_instruction_from_empty_address() {
        let memory = Memory::default();
        let addr = M31::from(10);

        let result = memory.get_instruction(addr);
        assert_eq!(result, QM31::zero());
    }

    #[test]
    fn test_get_data() {
        let mut memory = Memory::default();
        let addr = M31::from(42);
        // Create a QM31 value that represents a base field element (only first component non-zero)
        let value = QM31::from_m31(M31::from(123), M31::zero(), M31::zero(), M31::zero());

        memory.insert(addr, value);

        let retrieved = memory.get_data(addr);
        assert_eq!(retrieved, M31::from(123)); // Should get the projected M31 value
    }

    #[test]
    fn test_get_data_from_empty_address() {
        let memory = Memory::default();
        let addr = M31::from(10);

        let result = memory.get_data(addr);
        assert_eq!(result, M31::zero());
    }

    #[test]
    #[should_panic(expected = "assertion failed: qm31_value.1.is_zero()")]
    fn test_get_data_panic_on_non_base_field() {
        let mut memory = Memory::default();
        let addr = M31::from(42);
        // Create a QM31 value that is NOT in the base field
        let value = QM31::from_m31(M31::zero(), M31::zero(), M31::from(123), M31::zero());

        memory.insert(addr, value);
        memory.get_data(addr);
    }

    #[test]
    fn test_insert_instruction_auto_resize() {
        let mut memory = Memory::default();
        let addr = M31::from(100);
        let value = QM31::from_m31(M31::from(42), M31::zero(), M31::zero(), M31::zero());

        memory.insert(addr, value);

        assert_eq!(memory.data.len(), 101); // Should resize to address + 1
        assert_eq!(memory.get_instruction(addr), value);
    }

    #[test]
    fn test_validate_address() {
        // Test valid addresses (within 2^30 limit)
        Memory::validate_address(M31::from(100));
        Memory::validate_address(M31::from(1_000_000));
        Memory::validate_address(M31::from((1 << MAX_MEMORY_SIZE_BITS) - 1));
        Memory::validate_address(M31::from(1 << MAX_MEMORY_SIZE_BITS));
    }

    #[test]
    #[should_panic(expected = "Max memory size is 2 ** 30; got address: ")]
    fn test_validate_address_panic() {
        // Test address that exceeds the limit - should panic
        Memory::validate_address(M31::from((1 << MAX_MEMORY_SIZE_BITS) + 1));
    }

    #[test]
    fn test_insert_slice() {
        let mut memory = Memory::default();
        let start_addr = M31::from(10);
        let values = vec![
            QM31::from_m31(M31::from(1), M31::zero(), M31::zero(), M31::zero()),
            QM31::from_m31(M31::from(2), M31::zero(), M31::zero(), M31::zero()),
            QM31::from_m31(M31::from(3), M31::zero(), M31::zero(), M31::zero()),
        ];

        memory.insert_slice(start_addr, &values);

        assert_eq!(
            memory.get_instruction(M31::from(10)),
            QM31::from_m31(M31::from(1), M31::zero(), M31::zero(), M31::zero())
        );
        assert_eq!(
            memory.get_instruction(M31::from(11)),
            QM31::from_m31(M31::from(2), M31::zero(), M31::zero(), M31::zero())
        );
        assert_eq!(
            memory.get_instruction(M31::from(12)),
            QM31::from_m31(M31::from(3), M31::zero(), M31::zero(), M31::zero())
        );
    }

    #[test]
    #[should_panic(expected = "Max memory size is 2 ** 30")]
    fn test_insert_slice_start_addr_out_of_bounds() {
        let mut memory = Memory::default();
        let invalid_addr = M31::from((1 << MAX_MEMORY_SIZE_BITS) + 1);
        let values = vec![QM31::zero()];
        memory.insert_slice(invalid_addr, &values);
    }

    #[test]
    #[should_panic(expected = "Max memory size is 2 ** 30")]
    fn test_insert_slice_end_addr_out_of_bounds() {
        let mut memory = Memory::default();
        let start_addr = M31::from((1 << MAX_MEMORY_SIZE_BITS) - 5);
        let values = vec![QM31::zero(); 10]; // Would exceed limit
        memory.insert_slice(start_addr, &values);
    }

    #[test]
    fn test_extend() {
        let mut memory = Memory::default();
        let values = vec![
            QM31::from_m31(M31::from(10), M31::zero(), M31::zero(), M31::zero()),
            QM31::from_m31(M31::from(20), M31::zero(), M31::zero(), M31::zero()),
            QM31::from_m31(M31::from(30), M31::zero(), M31::zero(), M31::zero()),
        ];

        memory.extend(values);

        assert_eq!(memory.data.len(), 3);
        assert_eq!(memory.get_instruction(M31::from(0)), M31::from(10).into());
        assert_eq!(memory.get_instruction(M31::from(1)), M31::from(20).into());
        assert_eq!(memory.get_instruction(M31::from(2)), M31::from(30).into());
    }

    #[test]
    fn test_from_iter() {
        let values = vec![
            QM31::from_m31(M31::from(100), M31::zero(), M31::zero(), M31::zero()),
            QM31::from_m31(M31::from(200), M31::zero(), M31::zero(), M31::zero()),
        ];

        let memory: Memory = values.into_iter().collect();

        assert_eq!(memory.data.len(), 2);
        assert_eq!(memory.get_instruction(M31::from(0)), M31::from(100).into());
        assert_eq!(memory.get_instruction(M31::from(1)), M31::from(200).into());
    }
}
