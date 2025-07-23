use paste::paste;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InstructionError {
    #[error("Invalid opcode: {0}")]
    InvalidOpcode(M31),
    #[error("Size mismatch for instruction: expected {expected}, found {found}")]
    SizeMismatch { expected: usize, found: usize },
}

// Macro to define the Instruction enum with all variants and their implementations
macro_rules! define_instruction {
    (
        $(
            $variant:ident = $opcode:literal, $mem_access:literal,
            fields: [$($field:ident),*],
            size: $size:literal
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
        }

        impl TryFrom<&[M31]> for Instruction {
            type Error = InstructionError;

            fn try_from(values: &[M31]) -> Result<Self, Self::Error> {
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

        impl TryFrom<SmallVec<[M31; 5]>> for Instruction {
            type Error = InstructionError;

            #[inline(always)]
            fn try_from(values: SmallVec<[M31; 5]>) -> Result<Self, Self::Error> {
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
        const MAX_OPCODE: u32 = {
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
    // Arithmetic operations
    StoreAddFpFp = 0, 3, fields: [src0_off, src1_off, dst_off], size: 4;     // [fp + dst_off] = [fp + src0_off] + [fp + src1_off]
    StoreAddFpImm = 1, 2, fields: [src_off, imm, dst_off], size: 4;          // [fp + dst_off] = [fp + src_off] + imm
    StoreSubFpFp = 2, 3, fields: [src0_off, src1_off, dst_off], size: 4;     // [fp + dst_off] = [fp + src0_off] - [fp + src1_off]
    StoreSubFpImm = 3, 2, fields: [src_off, imm, dst_off], size: 4;          // [fp + dst_off] = [fp + src_off] - imm

    // Memory operations
    StoreDoubleDerefFp = 4, 3, fields: [base_off, offset, dst_off], size: 4; // [fp + dst_off] = [[fp + base_off] + offset]
    StoreImm = 5, 1, fields: [imm, dst_off], size: 3;                        // [fp + dst_off] = imm

    // Multiplication/Division
    StoreMulFpFp = 6, 3, fields: [src0_off, src1_off, dst_off], size: 4;    // [fp + dst_off] = [fp + src0_off] * [fp + src1_off]
    StoreMulFpImm = 7, 2, fields: [src_off, imm, dst_off], size: 4;         // [fp + dst_off] = [fp + src_off] * imm
    StoreDivFpFp = 8, 3, fields: [src0_off, src1_off, dst_off], size: 4;    // [fp + dst_off] = [fp + src0_off] / [fp + src1_off]
    StoreDivFpImm = 9, 2, fields: [src_off, imm, dst_off], size: 4;         // [fp + dst_off] = [fp + src_off] / imm

    // Call operations
    CallAbsImm = 10, 2, fields: [frame_off, target], size: 3;               // call abs imm
    Ret = 11, 2, fields: [], size: 1;                                       // ret

    // Jump operations
    JmpAbsImm = 12, 0, fields: [target], size: 2;                           // jmp abs imm
    JmpRelImm = 13, 0, fields: [offset], size: 2;                           // jmp rel imm

    // Conditional jumps
    JnzFpImm = 14, 1, fields: [cond_off, offset], size: 3;                  // jmp rel imm if [fp + cond_off] != 0

    // U32 instructions
    U32StoreAddFpImm = 15, 4, fields: [src_off, imm_hi, imm_lo, dst_off], size: 5 // u32([fp + dst_off], [fp + dst_off + 1]) = u32([fp + src_off], [fp + src_off + 1]) + u32(imm_lo, imm_hi)
);

impl From<Instruction> for Vec<M31> {
    fn from(instruction: Instruction) -> Self {
        instruction.to_m31_vec()
    }
}

impl From<&Instruction> for Vec<M31> {
    fn from(instruction: &Instruction) -> Self {
        instruction.to_m31_vec()
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
            1..=5 => {
                let mut m31_array = [M31::from(0); 5];
                for (i, s) in hex_strings.iter().enumerate() {
                    m31_array[i] = u32::from_str_radix(s.trim_start_matches("0x"), 16)
                        .map(M31::from)
                        .map_err(de::Error::custom)?;
                }
                Self::try_from(&m31_array[..hex_strings.len()]).map_err(de::Error::custom)
            }
            _ => Err(de::Error::custom(
                "Instruction too large (max 5 M31 elements)",
            )),
        }
    }
}
