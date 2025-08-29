use paste::paste;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    Felt,
    U32,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum InstructionError {
    #[error("Invalid opcode: {0}")]
    InvalidOpcode(M31),
    #[error("Size mismatch for instruction: expected {expected}, found {found}")]
    SizeMismatch { expected: usize, found: usize },
    #[error("Assertion failed: {0} != {1}")]
    AssertionFailed(M31, M31),
    #[error("Invalid instruction type: {0}")]
    InvalidInstructionType(&'static str),
}

pub const INSTRUCTION_MAX_SIZE: usize = 5;
// User-facing marker for field kinds used in the macro input.
// Note: This enum is only used for declarative purposes in the macro callsite;
// it is not stored in Instruction and has no runtime impact.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandType {
    Immediate,
    Memory(DataType),
}

// Helper macros to process field kinds
macro_rules! __mem_access_for_kind {
    (OperandType::Memory(DataType::Felt)) => {
        1
    };
    (OperandType::Memory(DataType::U32)) => {
        2
    };
    (OperandType::Immediate) => {
        0
    };
    ((OperandType::Memory(DataType::Felt))) => {
        1
    };
    ((OperandType::Memory(DataType::U32))) => {
        2
    };
    ((OperandType::Immediate)) => {
        0
    };
}

macro_rules! __mem_operand_kind_to_list {
    (OperandType::Memory(DataType::Felt)) => { DataType::Felt, };
    (OperandType::Memory(DataType::U32)) => { DataType::U32, };
    (OperandType::Immediate) => {};
    ((OperandType::Memory(DataType::Felt))) => { DataType::Felt, };
    ((OperandType::Memory(DataType::U32))) => { DataType::U32, };
    ((OperandType::Immediate)) => {};
}

// Macro to define the Instruction enum and implementations from a more descriptive spec.
macro_rules! instructions {
    (
        $(
            $variant:ident = $opcode:literal {
                $( $field:ident : $kind:tt ),* $(,)?
            } $(, implicit_operands: [$($implicit_kind:tt),* $(,)?])? $(;)?
        )*
    ) => {
        /// Cairo M instruction enum where each variant represents a specific opcode
        /// with its required named fields
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Instruction {
            $( $variant { $( $field: M31 ),* }, )*
        }

        // Generate opcode constants using paste
        paste! {
            $( pub const [<$variant:snake:upper>]: u32 = $opcode; )*
        }

        impl Instruction {
            /// Get the numeric opcode value for this instruction
            pub const fn opcode_value(&self) -> u32 {
                match self {
                    $( Self::$variant { .. } => $opcode, )*
                }
            }

            /// Get the size of this instruction in M31 elements (including opcode)
            pub const fn size_in_m31s(&self) -> usize {
                match self {
                    $( Self::$variant { .. } => 1usize $(+ { let _ = stringify!($field); 1usize })* , )*
                }
            }

            /// Get the size of this instruction in QM31 elements
            pub const fn size_in_qm31s(&self) -> u32 { self.size_in_m31s().div_ceil(4) as u32 }

            /// Get the size in M31 elements for a given opcode
            pub const fn size_in_m31s_for_opcode(opcode: u32) -> Option<usize> {
                match opcode {
                    $( $opcode => Some(1usize $(+ { let _ = stringify!($field); 1usize })*), )*
                    _ => None,
                }
            }

            /// Get the size in QM31 elements for a given opcode
            pub const fn size_in_qm31s_for_opcode(opcode: u32) -> Option<u32> {
                match Self::size_in_m31s_for_opcode(opcode) {
                    Some(size) => Some(size.div_ceil(4) as u32),
                    None => None,
                }
            }

            /// Get the number of memory accesses (as limbs) for this instruction's operands
            pub const fn memory_accesses(&self) -> usize {
                match self {
                    $(
                        Self::$variant { .. } => {
                            0usize
                            $( + __mem_access_for_kind!($kind) )*
                            $( $( + __mem_access_for_kind!($implicit_kind) )* )?
                        }
                    ),*
                }
            }

            /// Convert instruction to a vector of M31 values
            pub fn to_m31_vec(&self) -> Vec<M31> {
                let mut vec = vec![M31::from(self.opcode_value())];
                match self {
                    $( Self::$variant { $( $field ),* } => { $( vec.push(*$field); )* } ),*
                }
                vec
            }

            /// Convert instruction to a SmallVec of M31 values
            pub fn to_smallvec(&self) -> SmallVec<[M31; INSTRUCTION_MAX_SIZE]> {
                let mut vec = SmallVec::new();
                vec.push(M31::from(self.opcode_value()));
                match self {
                    $( Self::$variant { $( $field ),* } => { $( vec.push(*$field); )* } ),*
                }
                vec
            }

            /// Get the name of the instruction as a string
            pub const fn name(&self) -> &'static str {
                match self {
                    $( Self::$variant { .. } => stringify!($variant), )*
                }
            }

            /// Get all operands as a vector (excluding the opcode)
            pub fn operands(&self) -> Vec<M31> {
                let mut vec = Vec::with_capacity(self.size_in_m31s() - 1);
                match self {
                    $( Self::$variant { $( $field ),* } => { $( vec.push(*$field); )* } ),*
                }
                vec
            }

            /// Get the data types for each memory operand of this instruction
            /// NOTE: This placeholder returns an empty slice. The typed field metadata
            /// is present in the macro but operand type extraction is not used at runtime
            /// in this refactor step. If needed, we can reintroduce a generated static
            /// table per opcode.
            pub const fn operand_types(&self) -> &'static [DataType] {
                match self {
                    $( Self::$variant { .. } => &[], )*
                }
            }
        }

        impl TryFrom<SmallVec<[M31; INSTRUCTION_MAX_SIZE]>> for Instruction {
            type Error = InstructionError;

            #[inline(always)]
            fn try_from(values: SmallVec<[M31; INSTRUCTION_MAX_SIZE]>) -> Result<Self, Self::Error> {
                let (opcode_m31, operands) = values
                    .split_first()
                    .ok_or(InstructionError::SizeMismatch { expected: 1, found: 0 })?;
                let opcode_u32 = opcode_m31.0;

                match opcode_u32 {
                    $(
                        $opcode => {
                            if let [$( $field ),*] = operands {
                                Ok(Self::$variant { $( $field: *$field ),* })
                            } else {
                                Err(InstructionError::SizeMismatch {
                                    expected: (1usize $(+ { let _ = stringify!($field); 1usize })*) - 1,
                                    found: operands.len(),
                                })
                            }
                        }
                    )*
                    _ => Err(InstructionError::InvalidOpcode(*opcode_m31)),
                }
            }
        }

        // Generate the maximum opcode value
        pub const MAX_OPCODE: u32 = {
            let opcodes = [$($opcode),*];
            let mut max = 0;
            let mut i = 0;
            while i < opcodes.len() {
                if opcodes[i] > max { max = opcodes[i]; }
                i += 1;
            }
            max
        };

        /// Const lookup table for instruction sizes by opcode.
        pub const OPCODE_SIZE_TABLE: [Option<usize>; (MAX_OPCODE + 1) as usize] = {
            let mut table = [None; (MAX_OPCODE + 1) as usize];
            $( table[$opcode as usize] = Some(1usize $(+ { let _ = stringify!($field); 1usize })*); )*
            table
        };
    };
}

/// Extracts fields from a specific instruction variant or returns an InvalidOpcode error.
///
/// This macro simplifies instruction decoding by handling the boilerplate of matching
/// and error handling. It automatically dereferences the extracted fields.
///
/// # Panics
/// The macro generates a `return` statement, so it must be used inside a function
/// that returns a `Result<_, InstructionExecutionError>`.
///
/// # Usage
///
/// ## Extracting multiple fields into a tuple:
/// ```ignore
/// let (cond_off, offset) = extract_as!(instruction, JnzFpImm, (cond_off, offset));
/// ```
/// expands to:
/// ```ignore
/// let (cond_off, offset) = match instruction {
///     Instruction::JnzFpImm { cond_off, offset } => (*cond_off, *offset),
///     _ => return Err(InstructionExecutionError::InvalidInstructionType),
/// };
/// ```
///
/// ## Extracting a single field:
/// ```ignore
/// let target = extract_as!(instruction, JmpAbsImm, target);
/// ```
/// expands to:
/// ```ignore
/// let target = match instruction {
///     Instruction::JmpAbsImm { target } => *target,
///     _ => return Err(InstructionExecutionError::InvalidInstructionType),
/// };
/// ```
#[macro_export]
macro_rules! extract_as {
    // Case 1: Extracting multiple fields into a tuple.
    // e.g., extract_as!(instruction, JnzFpImm, (cond_off, offset))
    ($instruction:expr, $variant:ident, ($($field:ident),+)) => {
        match $instruction {
            $crate::Instruction::$variant { $($field),+ } => {
                // Creates a tuple of the dereferenced fields: (*cond_off, *offset)
                ($(*$field),+)
            },
            _ => {
                return Err($crate::InstructionError::InvalidInstructionType(stringify!($variant)).into());
            }
        }
    };

    // Case 2: Extracting a single field.
    // e.g., extract_as!(instruction, JmpAbsImm, target)
    ($instruction:expr, $variant:ident, $field:ident) => {
        match $instruction {
            $crate::Instruction::$variant { $field } => {
                // Dereferences the single field: *target
                *$field
            },
            _ => {
                return Err($crate::InstructionError::InvalidInstructionType(stringify!($variant)).into());
            }
        }
    };

    // Case 3: Validating instruction variant with no fields (like Ret).
    // e.g., extract_as!(instruction, Ret)
    ($instruction:expr, $variant:ident) => {
        match $instruction {
            $crate::Instruction::$variant { .. } => {
                // No fields to extract, just validates the variant
            },
            _ => {
                return Err($crate::InstructionError::InvalidInstructionType(stringify!($variant)).into());
            }
        }
    };
}

// Define all instructions with their opcodes and typed fields
instructions! {
    // Arithmetic operations: order matters for the prover, see store_fp_fp.rs
    // [fp + dst_off] = [fp + src0_off] + [fp + src1_off]
    StoreAddFpFp = 0 {
        src0_off: (OperandType::Memory(DataType::Felt)),
        src1_off: (OperandType::Memory(DataType::Felt)),
        dst_off: (OperandType::Memory(DataType::Felt)),
    };
    // [fp + dst_off] = [fp + src0_off] - [fp + src1_off]
    StoreSubFpFp = 1 {
        src0_off: (OperandType::Memory(DataType::Felt)),
        src1_off: (OperandType::Memory(DataType::Felt)),
        dst_off: (OperandType::Memory(DataType::Felt)),
    };
    // [fp + dst_off] = [fp + src0_off] * [fp + src1_off]
    StoreMulFpFp = 2 {
        src0_off: (OperandType::Memory(DataType::Felt)),
        src1_off: (OperandType::Memory(DataType::Felt)),
        dst_off: (OperandType::Memory(DataType::Felt)),
    };
    // [fp + dst_off] = [fp + src0_off] / [fp + src1_off]
    StoreDivFpFp = 3 {
        src0_off: (OperandType::Memory(DataType::Felt)),
        src1_off: (OperandType::Memory(DataType::Felt)),
        dst_off: (OperandType::Memory(DataType::Felt)),
    };

    // Arithmetic operations with immediate: order matters for the prover, see store_fp_imm.rs
    // [fp + dst_off] = [fp + src_off] + imm
    StoreAddFpImm = 4 {
        src_off: (OperandType::Memory(DataType::Felt)),
        imm: (OperandType::Immediate),
        dst_off: (OperandType::Memory(DataType::Felt)),
    };
    // [fp + dst_off] = [fp + src_off] * imm
    StoreMulFpImm = 6 {
        src_off: (OperandType::Memory(DataType::Felt)),
        imm: (OperandType::Immediate),
        dst_off: (OperandType::Memory(DataType::Felt)),
    };

    // Comparison operations
    // [fp + dst_off] = [fp + src_off] < imm
    StoreLowerThanFpImm = 48 {
        src_off: (OperandType::Memory(DataType::Felt)),
        imm: (OperandType::Immediate),
        dst_off: (OperandType::Memory(DataType::Felt)),
    };

    // Assertions
    // assert [fp + src0_off] == [fp + src1_off]
    AssertEqFpFp = 49 {
        src0_off: (OperandType::Memory(DataType::Felt)),
        src1_off: (OperandType::Memory(DataType::Felt)),
    };
    // assert [fp + src_off] == imm
    AssertEqFpImm = 50 {
        src_off: (OperandType::Memory(DataType::Felt)),
        imm: (OperandType::Immediate),
    };

    // Memory operations
    // [fp + dst_off] = [[fp + base_off] + imm]
    StoreDoubleDerefFp = 8 {
        base_off: (OperandType::Memory(DataType::Felt)),
        imm: (OperandType::Immediate),
        dst_off: (OperandType::Memory(DataType::Felt)),
    };
    // [fp + dst_off] = [[fp + base_off] + [fp + offset_off]]
    StoreDoubleDerefFpFp = 42 {
        base_off: (OperandType::Memory(DataType::Felt)),
        offset_off: (OperandType::Memory(DataType::Felt)),
        dst_off: (OperandType::Memory(DataType::Felt)),
    };
    // [fp + dst_off] = imm
    StoreImm = 9 {
        imm: (OperandType::Immediate),
        dst_off: (OperandType::Memory(DataType::Felt)),
    };
    // [fp + dst_off] = fp + imm
    StoreFpImm = 43 {
        imm: (OperandType::Immediate),
        dst_off: (OperandType::Memory(DataType::Felt)),
    };

    // Call operations
    // call abs imm
    CallAbsImm = 10 {
        frame_off: (OperandType::Immediate),
        target: (OperandType::Immediate),
    };
    // ret
    Ret = 11 {};

    // Jump operations
    // jmp abs imm
    JmpAbsImm = 12 { target: (OperandType::Immediate) };
    // jmp rel imm
    JmpRelImm = 13 { offset: (OperandType::Immediate) };

    // Conditional jumps
    // jmp rel imm if [fp + cond_off] != 0
    JnzFpImm = 14 {
        cond_off: (OperandType::Memory(DataType::Felt)),
        offset: (OperandType::Immediate),
    };

    // U32 operations with FP operands
    // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src0_off], [fp + src0_off + 1]) + u32([fp + src1_off], [fp + src1_off + 1])
    U32StoreAddFpFp = 15 {
        src0_off: (OperandType::Memory(DataType::U32)),
        src1_off: (OperandType::Memory(DataType::U32)),
        dst_off: (OperandType::Memory(DataType::U32)),
    };
    // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src0_off], [fp + src0_off + 1]) - u32([fp + src1_off], [fp + src1_off + 1])
    U32StoreSubFpFp = 16 {
        src0_off: (OperandType::Memory(DataType::U32)),
        src1_off: (OperandType::Memory(DataType::U32)),
        dst_off: (OperandType::Memory(DataType::U32)),
    };
    // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src0_off], [fp + src0_off + 1]) * u32([fp + src1_off], [fp + src1_off + 1])
    U32StoreMulFpFp = 17 {
        src0_off: (OperandType::Memory(DataType::U32)),
        src1_off: (OperandType::Memory(DataType::U32)),
        dst_off: (OperandType::Memory(DataType::U32)),
    };
    // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src0_off], [fp + src0_off + 1]) / u32([fp + src1_off], [fp + src1_off + 1])
    U32StoreDivFpFp = 18 {
        src0_off: (OperandType::Memory(DataType::U32)),
        src1_off: (OperandType::Memory(DataType::U32)),
        dst_off: (OperandType::Memory(DataType::U32)),
    };

    // U32 operations with immediate
    // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src_off], [fp + src_off + 1]) + u32(imm_lo, imm_hi)
    U32StoreAddFpImm = 19 {
        src_off: (OperandType::Memory(DataType::U32)),
        imm_lo: (OperandType::Immediate),
        imm_hi: (OperandType::Immediate),
        dst_off: (OperandType::Memory(DataType::U32)),
    };
    // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src_off], [fp + src_off + 1]) * u32(imm_lo, imm_hi)
    U32StoreMulFpImm = 21 {
        src_off: (OperandType::Memory(DataType::U32)),
        imm_lo: (OperandType::Immediate),
        imm_hi: (OperandType::Immediate),
        dst_off: (OperandType::Memory(DataType::U32)),
    };
    // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src_off], [fp + src_off + 1]) / u32(imm_lo, imm_hi)
    U32StoreDivFpImm = 22 {
        src_off: (OperandType::Memory(DataType::U32)),
        imm_lo: (OperandType::Immediate),
        imm_hi: (OperandType::Immediate),
        dst_off: (OperandType::Memory(DataType::U32)),
    };

    // U32 Memory operations
    // u32([fp + dst_off], [fp + dst_off + 1]) = u32(imm_lo, imm_hi)
    U32StoreImm = 23 {
        imm_lo: (OperandType::Immediate),
        imm_hi: (OperandType::Immediate),
        dst_off: (OperandType::Memory(DataType::U32)),
    };

    // U32 Comparison operations
    // [fp + dst_off] = u32([fp + src0_off], [fp + src0_off + 1]) == u32([fp + src1_off], [fp + src1_off + 1])
    U32StoreEqFpFp = 24 {
        src0_off: (OperandType::Memory(DataType::U32)),
        src1_off: (OperandType::Memory(DataType::U32)),
        dst_off: (OperandType::Memory(DataType::Felt)),
    };
    // [fp + dst_off] = u32([fp + src0_off], [fp + src0_off + 1]) < u32([fp + src1_off], [fp + src1_off + 1])
    U32StoreLtFpFp = 28 {
        src0_off: (OperandType::Memory(DataType::U32)),
        src1_off: (OperandType::Memory(DataType::U32)),
        dst_off: (OperandType::Memory(DataType::Felt)),
    };

    // U32 Comparison operations with immediate
    // [fp + dst_off] = u32([fp + src_off], [fp + src_off + 1]) == u32(imm_lo, imm_hi)
    U32StoreEqFpImm = 30 {
        src_off: (OperandType::Memory(DataType::U32)),
        imm_lo: (OperandType::Immediate),
        imm_hi: (OperandType::Immediate),
        dst_off: (OperandType::Memory(DataType::Felt)),
    };
    // [fp + dst_off] = u32([fp + src_off], [fp + src_off + 1]) < u32(imm_lo, imm_hi)
    U32StoreLtFpImm = 34 {
        src_off: (OperandType::Memory(DataType::U32)),
        imm_lo: (OperandType::Immediate),
        imm_hi: (OperandType::Immediate),
        dst_off: (OperandType::Memory(DataType::Felt)),
    };

    // U32 Bitwise operations
    // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src0_off], [fp + src0_off + 1]) &/|/^ u32([fp + src1_off], [fp + src1_off + 1])
    U32StoreAndFpFp = 36 {
        src0_off: (OperandType::Memory(DataType::U32)),
        src1_off: (OperandType::Memory(DataType::U32)),
        dst_off: (OperandType::Memory(DataType::U32)),
    };
    U32StoreOrFpFp = 37 {
        src0_off: (OperandType::Memory(DataType::U32)),
        src1_off: (OperandType::Memory(DataType::U32)),
        dst_off: (OperandType::Memory(DataType::U32)),
    };
    U32StoreXorFpFp = 38 {
        src0_off: (OperandType::Memory(DataType::U32)),
        src1_off: (OperandType::Memory(DataType::U32)),
        dst_off: (OperandType::Memory(DataType::U32)),
    };

    // U32 Bitwise operations with immediate
    U32StoreAndFpImm = 39 {
        src_off: (OperandType::Memory(DataType::U32)),
        imm_lo: (OperandType::Immediate),
        imm_hi: (OperandType::Immediate),
        dst_off: (OperandType::Memory(DataType::U32)),
    };
    U32StoreOrFpImm = 40 {
        src_off: (OperandType::Memory(DataType::U32)),
        imm_lo: (OperandType::Immediate),
        imm_hi: (OperandType::Immediate),
        dst_off: (OperandType::Memory(DataType::U32)),
    };
    U32StoreXorFpImm = 41 {
        src_off: (OperandType::Memory(DataType::U32)),
        imm_lo: (OperandType::Immediate),
        imm_hi: (OperandType::Immediate),
        dst_off: (OperandType::Memory(DataType::U32)),
    };

    // Reverse double deref operations - store TO computed addresses
    // [[fp + base_off] + imm] = [fp + src_off]
    StoreToDoubleDerefFpImm = 44 {
        src_off: (OperandType::Memory(DataType::Felt)),
        imm: (OperandType::Immediate),
        base_off: (OperandType::Memory(DataType::Felt)),
    };
    // [[fp + base_off] + [fp + offset_off]] = [fp + src_off]
    StoreToDoubleDerefFpFp = 45 {
        src_off: (OperandType::Memory(DataType::Felt)),
        base_off: (OperandType::Memory(DataType::Felt)),
        offset_off: (OperandType::Memory(DataType::Felt)),
    };

    // Print operations for debugging
    PrintM31 = 46 { offset: (OperandType::Memory(DataType::Felt)) };
    PrintU32 = 47 { offset: (OperandType::Memory(DataType::U32)) };
}

impl From<Instruction> for SmallVec<[M31; INSTRUCTION_MAX_SIZE]> {
    fn from(instruction: Instruction) -> Self {
        instruction.to_smallvec()
    }
}

impl From<&Instruction> for SmallVec<[M31; INSTRUCTION_MAX_SIZE]> {
    fn from(instruction: &Instruction) -> Self {
        instruction.to_smallvec()
    }
}

impl Instruction {
    /// Convert instruction to QM31 values for memory storage
    /// Instructions are padded with zeros to align to QM31 boundaries
    pub fn to_qm31_vec(&self) -> Vec<QM31> {
        self.to_m31_vec()
            .chunks(4)
            .map(|chunk| {
                let mut m31_array = [M31::from(0); 4];
                chunk
                    .iter()
                    .enumerate()
                    .for_each(|(i, &val)| m31_array[i] = val);
                QM31::from_m31_array(m31_array)
            })
            .collect()
    }
}

// Serialize instruction as JSON array of hex strings
impl Serialize for Instruction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;
        let vec = self.to_m31_vec();
        let mut seq = serializer.serialize_seq(Some(vec.len()))?;

        for val in &vec {
            seq.serialize_element(&format!("0x{:x}", val.0))?;
        }
        seq.end()
    }
}

// Deserialize instruction from JSON array
impl<'de> Deserialize<'de> for Instruction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        let hex_strings: Vec<String> = Deserialize::deserialize(deserializer)?;

        // Validate instruction size and convert to M31 array
        match hex_strings.len() {
            0 => Err(de::Error::custom("Instruction cannot be empty")),
            1..=INSTRUCTION_MAX_SIZE => {
                let mut values = SmallVec::<[M31; INSTRUCTION_MAX_SIZE]>::new();
                for s in hex_strings {
                    let m31 = u32::from_str_radix(s.trim_start_matches("0x"), 16)
                        .map(M31::from)
                        .map_err(de::Error::custom)?;
                    values.push(m31);
                }
                Self::try_from(values).map_err(de::Error::custom)
            }
            _ => Err(de::Error::custom(format!(
                "Instruction too large (max {} M31 elements)",
                INSTRUCTION_MAX_SIZE
            ))),
        }
    }
}
