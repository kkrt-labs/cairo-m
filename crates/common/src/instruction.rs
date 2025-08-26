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

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InstructionError {
    #[error("Invalid opcode: {0}")]
    InvalidOpcode(M31),
    #[error("Size mismatch for instruction: expected {expected}, found {found}")]
    SizeMismatch { expected: usize, found: usize },
    #[error("Assertion failed: {0} != {1}")]
    AssertionFailed(M31, M31),
}

pub const INSTRUCTION_MAX_SIZE: usize = 5;

// Macro to define the Instruction enum with all variants and their implementations
macro_rules! define_instruction {
    (
        $(
            $variant:ident = $opcode:literal,
            $mem_access:literal,
            fields: [$($field:ident),*],
            size: $size:literal,
            operands: [$($operand_type:ident),*]
        );*
    ) => {
        /// Cairo M instruction enum where each variant represents a specific opcode
        /// with its required named fields
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Instruction {
            $(
                $variant { $($field: M31),* },
            )*
        }

        // Generate opcode constants using paste
        paste! {
            $(
                pub const [<$variant:snake:upper>]: u32 = $opcode;
            )*
        }

        impl Instruction {
            /// Get the numeric opcode value for this instruction
            pub const fn opcode_value(&self) -> u32 {
                match self {
                    $(
                        Self::$variant { .. } => $opcode,
                    )*
                }
            }

            /// Get the size of this instruction in M31 elements (including opcode)
            pub const fn size_in_m31s(&self) -> usize {
                match self {
                    $(
                        Self::$variant { .. } => $size,
                    )*
                }
            }

            /// Get the size of this instruction in QM31 elements
            pub const fn size_in_qm31s(&self) -> u32 {
                self.size_in_m31s().div_ceil(4) as u32
            }

            /// Get the size in M31 elements for a given opcode
            pub const fn size_in_m31s_for_opcode(opcode: u32) -> Option<usize> {
                match opcode {
                    $(
                        $opcode => Some($size),
                    )*
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

            /// Get the number of memory accesses for this instruction
            pub const fn memory_accesses(&self) -> usize {
                match self {
                    $(
                        Self::$variant { .. } => $mem_access,
                    )*
                }
            }

            /// Convert instruction to a vector of M31 values
            pub fn to_m31_vec(&self) -> Vec<M31> {
                let mut vec = vec![M31::from(self.opcode_value())];
                match self {
                    $(
                        Self::$variant { $($field),* } => {
                            $(
                                vec.push(*$field);
                            )*
                        }
                    )*
                }
                vec
            }

            /// Convert instruction to a SmallVec of M31 values
            pub fn to_smallvec(&self) -> SmallVec<[M31; INSTRUCTION_MAX_SIZE]> {
                let mut vec = SmallVec::new();
                vec.push(M31::from(self.opcode_value()));
                match self {
                    $(
                        Self::$variant { $($field),* } => {
                            $(
                                vec.push(*$field);
                            )*
                        }
                    )*
                }
                vec
            }

            /// Get the name of the instruction as a string
            pub const fn name(&self) -> &'static str {
                match self {
                    $(
                        Self::$variant { .. } => stringify!($variant),
                    )*
                }
            }

            /// Get all operands as a vector (excluding the opcode)
            pub fn operands(&self) -> Vec<M31> {
                let mut vec = Vec::with_capacity(self.size_in_m31s() - 1);
                match self {
                    $(
                        Self::$variant { $($field),* } => {
                            $(
                                vec.push(*$field);
                            )*
                        }
                    )*
                }
                vec
            }

            /// Get the data types for each memory operand of this instruction
            pub const fn operand_types(&self) -> &'static [DataType] {
                use DataType::*;
                match self {
                    $(
                        Self::$variant { .. } => &[$($operand_type),*],
                    )*
                }
            }
        }

        impl TryFrom<SmallVec<[M31; INSTRUCTION_MAX_SIZE]>> for Instruction {
            type Error = InstructionError;

            #[inline(always)]
            fn try_from(values: SmallVec<[M31; INSTRUCTION_MAX_SIZE]>) -> Result<Self, Self::Error> {
                let (opcode_m31, operands) = values.split_first()
                    .ok_or(InstructionError::SizeMismatch { expected: 1, found: 0 })?;
                let opcode_u32 = opcode_m31.0;

                match opcode_u32 {
                    $(
                        $opcode => {
                            if let [$($field),*] = operands {
                                Ok(Self::$variant {
                                    $(
                                        $field: *$field,
                                    )*
                                })
                            } else {
                                Err(InstructionError::SizeMismatch {
                                    expected: $size - 1,
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
                if opcodes[i] > max {
                    max = opcodes[i];
                }
                i += 1;
            }
            max
        };

        /// Const lookup table for instruction sizes by opcode.
        /// This avoids the overhead of match statements in hot paths.
        /// Automatically generated from the instruction definitions.
        pub const OPCODE_SIZE_TABLE: [Option<usize>; (MAX_OPCODE + 1) as usize] = {
            let mut table = [None; (MAX_OPCODE + 1) as usize];
            $(
                table[$opcode as usize] = Some($size);
            )*
            table
        };
    };
}

// Define all instructions with their opcodes, memory accesses, fields, and sizes
define_instruction!(
    // Arithmetic operations: order matters for the prover, see store_fp_fp.rs
    StoreAddFpFp = 0, 3, fields: [src0_off, src1_off, dst_off], size: 4, operands: [Felt, Felt, Felt];     // [fp + dst_off] = [fp + src0_off] + [fp + src1_off]
    StoreSubFpFp = 1, 3, fields: [src0_off, src1_off, dst_off], size: 4, operands: [Felt, Felt, Felt];     // [fp + dst_off] = [fp + src0_off] - [fp + src1_off]
    StoreMulFpFp = 2, 3, fields: [src0_off, src1_off, dst_off], size: 4, operands: [Felt, Felt, Felt];     // [fp + dst_off] = [fp + src0_off] * [fp + src1_off]
    StoreDivFpFp = 3, 3, fields: [src0_off, src1_off, dst_off], size: 4, operands: [Felt, Felt, Felt];     // [fp + dst_off] = [fp + src0_off] / [fp + src1_off]

    // Arithmetic operations with immediate: order matters for the prover, see store_fp_imm.rs
    StoreAddFpImm = 4, 2, fields: [src_off, imm, dst_off], size: 4, operands: [Felt, Felt];                // [fp + dst_off] = [fp + src_off] + imm
    StoreSubFpImm = 5, 2, fields: [src_off, imm, dst_off], size: 4, operands: [Felt, Felt];                // [fp + dst_off] = [fp + src_off] - imm
    StoreMulFpImm = 6, 2, fields: [src_off, imm, dst_off], size: 4, operands: [Felt, Felt];                // [fp + dst_off] = [fp + src_off] * imm
    StoreDivFpImm = 7, 2, fields: [src_off, imm, dst_off], size: 4, operands: [Felt, Felt];                // [fp + dst_off] = [fp + src_off] / imm

    // Memory operations
    StoreDoubleDerefFp = 8, 3, fields: [base_off, imm, dst_off], size: 4, operands: [Felt, Felt, Felt]; // [fp + dst_off] = [[fp + base_off] + imm]
    StoreDoubleDerefFpFp = 42, 3, fields: [base_off, offset_off, dst_off], size: 4, operands: [Felt, Felt, Felt]; // [fp + dst_off] = [[fp + base_off] + [fp + offset_off]]
    StoreImm = 9, 1, fields: [imm, dst_off], size: 3, operands: [Felt];                                    // [fp + dst_off] = imm
    StoreFpImm = 43, 2, fields: [imm, dst_off], size: 3, operands: [Felt];                                  // [fp + dst_off] = fp + imm

    // Call operations
    CallAbsImm = 10, 2, fields: [frame_off, target], size: 3, operands: [Felt, Felt];                      // call abs imm
    Ret = 11, 2, fields: [], size: 1, operands: [Felt, Felt];                                              // ret

    // Jump operations
    JmpAbsImm = 12, 0, fields: [target], size: 2, operands: [];                                           // jmp abs imm
    JmpRelImm = 13, 0, fields: [offset], size: 2, operands: [];                                           // jmp rel imm

    // Conditional jumps
    JnzFpImm = 14, 1, fields: [cond_off, offset], size: 3, operands: [Felt];                              // jmp rel imm if [fp + cond_off] != 0

    // U32 operations with FP operands
    U32StoreAddFpFp = 15, 6, fields: [src0_off, src1_off, dst_off], size: 4, operands: [U32, U32, U32];   // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src0_off], [fp + src0_off + 1]) + u32([fp + src1_off], [fp + src1_off + 1])
    U32StoreSubFpFp = 16, 6, fields: [src0_off, src1_off, dst_off], size: 4, operands: [U32, U32, U32];   // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src0_off], [fp + src0_off + 1]) - u32([fp + src1_off], [fp + src1_off + 1])
    U32StoreMulFpFp = 17, 6, fields: [src0_off, src1_off, dst_off], size: 4, operands: [U32, U32, U32];   // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src0_off], [fp + src0_off + 1]) * u32([fp + src1_off], [fp + src1_off + 1])
    U32StoreDivFpFp = 18, 6, fields: [src0_off, src1_off, dst_off], size: 4, operands: [U32, U32, U32];   // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src0_off], [fp + src0_off + 1]) / u32([fp + src1_off], [fp + src1_off + 1])

    // U32 operations with immediate
    U32StoreAddFpImm = 19, 4, fields: [src_off, imm_lo, imm_hi, dst_off], size: 5, operands: [U32, U32];  // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src_off], [fp + src_off + 1]) + u32(imm_lo, imm_hi)
    U32StoreSubFpImm = 20, 4, fields: [src_off, imm_lo, imm_hi, dst_off], size: 5, operands: [U32, U32];  // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src_off], [fp + src_off + 1]) - u32(imm_lo, imm_hi)
    U32StoreMulFpImm = 21, 4, fields: [src_off, imm_lo, imm_hi, dst_off], size: 5, operands: [U32, U32];  // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src_off], [fp + src_off + 1]) * u32(imm_lo, imm_hi)
    U32StoreDivFpImm = 22, 4, fields: [src_off, imm_lo, imm_hi, dst_off], size: 5, operands: [U32, U32];   // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src_off], [fp + src_off + 1]) / u32(imm_lo, imm_hi)

    // U32 Memory operations
    U32StoreImm = 23, 2, fields: [imm_lo, imm_hi, dst_off], size: 4, operands: [U32, U32];                             // u32([fp + dst_off], [fp + dst_off + 1]) = u32(imm_lo, imm_hi)

    // U32 Comparison operations
    U32StoreEqFpFp = 24, 5, fields: [src0_off, src1_off, dst_off], size: 4, operands: [U32, U32, Felt];                // [fp + dst_off] = u32([fp + src0_off], [fp + src0_off + 1]) == u32([fp + src1_off], [fp + src1_off + 1])
    U32StoreNeqFpFp = 25, 5, fields: [src0_off, src1_off, dst_off], size: 4, operands: [U32, U32, Felt];               // [fp + dst_off] = u32([fp + src0_off], [fp + src0_off + 1]) != u32([fp + src1_off], [fp + src1_off + 1])
    U32StoreGtFpFp = 26, 5, fields: [src0_off, src1_off, dst_off], size: 4, operands: [U32, U32, Felt];                // [fp + dst_off] = u32([fp + src0_off], [fp + src0_off + 1]) > u32([fp + src1_off], [fp + src1_off + 1])
    U32StoreGeFpFp = 27, 5, fields: [src0_off, src1_off, dst_off], size: 4, operands: [U32, U32, Felt];                // [fp + dst_off] = u32([fp + src0_off], [fp + src0_off + 1]) >= u32([fp + src1_off], [fp + src1_off + 1])
    U32StoreLtFpFp = 28, 5, fields: [src0_off, src1_off, dst_off], size: 4, operands: [U32, U32, Felt];                // [fp + dst_off] = u32([fp + src0_off], [fp + src0_off + 1]) < u32([fp + src1_off], [fp + src1_off + 1])
    U32StoreLeFpFp = 29, 5, fields: [src0_off, src1_off, dst_off], size: 4, operands: [U32, U32, Felt];                 // [fp + dst_off] = u32([fp + src0_off], [fp + src0_off + 1]) <= u32([fp + src1_off], [fp + src1_off + 1])

    // U32 Comparison operations with immediate
    U32StoreEqFpImm = 30, 4, fields: [src_off, imm_lo, imm_hi, dst_off], size: 5, operands: [U32, U32];  // [fp + dst_off] = u32([fp + src_off], [fp + src_off + 1]) == u32(imm_lo, imm_hi)
    U32StoreNeqFpImm = 31, 4, fields: [src_off, imm_lo, imm_hi, dst_off], size: 5, operands: [U32, U32];  // [fp + dst_off] = u32([fp + src_off], [fp + src_off + 1]) != u32(imm_lo, imm_hi)
    U32StoreGtFpImm = 32, 4, fields: [src_off, imm_lo, imm_hi, dst_off], size: 5, operands: [U32, U32];  // [fp + dst_off] = u32([fp + src_off], [fp + src_off + 1]) > u32(imm_lo, imm_hi)
    U32StoreGeFpImm = 33, 4, fields: [src_off, imm_lo, imm_hi, dst_off], size: 5, operands: [U32, U32];  // [fp + dst_off] = u32([fp + src_off], [fp + src_off + 1]) >= u32(imm_lo, imm_hi)
    U32StoreLtFpImm = 34, 4, fields: [src_off, imm_lo, imm_hi, dst_off], size: 5, operands: [U32, U32];  // [fp + dst_off] = u32([fp + src_off], [fp + src_off + 1]) < u32(imm_lo, imm_hi)
    U32StoreLeFpImm = 35, 4, fields: [src_off, imm_lo, imm_hi, dst_off], size: 5, operands: [U32, U32];  // [fp + dst_off] = u32([fp + src_off], [fp + src_off + 1]) <= u32(imm_lo, imm_hi)

    // U32 Bitwise operations
    U32StoreAndFpFp = 36, 5, fields: [src0_off, src1_off, dst_off], size: 4, operands: [U32, U32, U32];                // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src0_off], [fp + src0_off + 1]) & u32([fp + src1_off], [fp + src1_off + 1])
    U32StoreOrFpFp = 37, 5, fields: [src0_off, src1_off, dst_off], size: 4, operands: [U32, U32, U32];                 // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src0_off], [fp + src0_off + 1]) | u32([fp + src1_off], [fp + src1_off + 1])
    U32StoreXorFpFp = 38, 5, fields: [src0_off, src1_off, dst_off], size: 4, operands: [U32, U32, U32];                // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src0_off], [fp + src0_off + 1]) ^ u32([fp + src1_off], [fp + src1_off + 1])

    // U32 Bitwise operations with immediate
    U32StoreAndFpImm = 39, 4, fields: [src_off, imm_lo, imm_hi, dst_off], size: 5, operands: [U32, U32];  // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src_off], [fp + src_off + 1]) & u32(imm_lo, imm_hi)
    U32StoreOrFpImm = 40, 4, fields: [src_off, imm_lo, imm_hi, dst_off], size: 5, operands: [U32, U32];  // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src_off], [fp + src_off + 1]) | u32(imm_lo, imm_hi)
    U32StoreXorFpImm = 41, 4, fields: [src_off, imm_lo, imm_hi, dst_off], size: 5, operands: [U32, U32];  // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src_off], [fp + src_off + 1]) ^ u32(imm_lo, imm_hi)

    // 42 taken by StoreDoubleDerefFpFp; 43 taken by StoreFpImm
    // Reverse double deref operations - store TO computed addresses
    StoreToDoubleDerefFpImm = 44, 3, fields: [base_off, imm, src_off], size: 4, operands: [Felt, Felt, Felt]; // [[fp + base_off] + imm] = [fp + src_off]
    StoreToDoubleDerefFpFp = 45, 3, fields: [base_off, offset_off, src_off], size: 4, operands: [Felt, Felt, Felt]; // [[fp + base_off] + [fp + offset_off]] = [fp + src_off]

    // Print operations for debugging
    PrintM31 = 46, 1, fields: [offset], size: 2, operands: [Felt];                      // print [fp + offset] as M31
    PrintU32 = 47, 2, fields: [offset], size: 2, operands: [U32];                        // print u32([fp + offset], [fp + offset + 1])

    StoreLowerThanFpImm = 48, 2, fields: [src_off, imm, dst_off], size: 4, operands: [Felt, Felt]; // [fp + dst_off] = [fp + src_off] < imm
    AssertEqFpFp = 49, 2, fields: [src0_off, src1_off], size: 3, operands: [Felt, Felt]; // assert [fp + src0_off] == [fp + src1_off]
    AssertEqFpImm = 50, 1, fields: [src_off, imm], size: 3, operands: [Felt] // assert [fp + src_off] == imm
);

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
