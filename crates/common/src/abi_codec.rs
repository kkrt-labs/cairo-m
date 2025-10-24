use stwo::core::fields::m31::{M31, P};

/// Untyped input value that can be interpreted based on AbiType
#[derive(Debug, Clone, PartialEq)]
pub enum InputValue {
    /// A numeric value that can be interpreted as felt, u32, bool, or pointer based on AbiType
    Number(i64),
    /// An explicit boolean value
    Bool(bool),
    /// A list of values (for arrays or tuples)
    List(Vec<InputValue>),
    /// A struct with positional fields (field names come from AbiType)
    Struct(Vec<InputValue>),
    /// Unit value
    Unit,
}

impl TryFrom<InputValue> for M31 {
    type Error = AbiCodecError;
    fn try_from(value: InputValue) -> Result<Self, Self::Error> {
        match value {
            InputValue::Number(n) => Ok(m31_from_i64(n)),
            InputValue::Bool(b) => Ok(Self::from(if b { 1u32 } else { 0u32 })),
            _ => Err(AbiCodecError::TypeMismatch(format!(
                "Cannot convert {:?} to M31",
                value
            ))),
        }
    }
}

impl From<M31> for InputValue {
    fn from(value: M31) -> Self {
        Self::Number(value.0.into())
    }
}

impl From<u32> for InputValue {
    fn from(value: u32) -> Self {
        Self::Number(value.into())
    }
}

/// Typed output value decoded from memory
#[derive(Debug, Clone, PartialEq)]
pub enum CairoMValue {
    Felt(M31),
    Bool(bool),
    U32(u32),
    Pointer(M31),
    Tuple(Vec<CairoMValue>),
    Struct(Vec<(String, CairoMValue)>),
    Array(Vec<CairoMValue>),
    Unit,
}

impl TryFrom<CairoMValue> for M31 {
    type Error = AbiCodecError;
    fn try_from(value: CairoMValue) -> Result<Self, Self::Error> {
        match value {
            CairoMValue::Felt(m31) => Ok(m31),
            CairoMValue::Bool(b) => Ok(Self::from(if b { 1u32 } else { 0u32 })),
            CairoMValue::U32(u) => Ok(Self::from(u)),
            _ => Err(AbiCodecError::TypeMismatch(format!(
                "Cannot convert {:?} to M31",
                value
            ))),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AbiCodecError {
    #[error("Type/value mismatch at {0}")]
    TypeMismatch(String),
    #[error("Insufficient data while decoding")]
    InsufficientData,
    #[error("Dynamic arrays not supported in ABI ing/decoding")]
    DynamicArrayUnsupported,
    #[error("Unexpected trailing or insufficient return data")]
    TrailingOrInsufficientData,
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Convert an i64 to an M31
pub const fn m31_from_i64(n: i64) -> M31 {
    // Perform modular reduction over Z, then map to M31 safely.
    // Works for all i64, including large magnitude negatives.
    let p = P as i128;
    let mut m = (n as i128) % p;
    if m < 0 {
        m += p;
    }
    M31::reduce(m as u64)
}

/// Parse an argument string into an InputValue (supports nesting)
///
/// Supported grammar (positional structs):
///   Value := Number | Bool | Array | Tuple | Struct
///   Array := '[' (Value (',' Value)*)? ']'
///   Tuple := '(' (Value (',' Value)*)? ')'
///   Struct := '{' (Value (',' Value)*)? '}'
pub fn parse_cli_arg(s: &str) -> Result<InputValue, AbiCodecError> {
    struct Parser<'a> {
        src: &'a [u8],
        i: usize,
    }
    impl<'a> Parser<'a> {
        const fn new(s: &'a str) -> Self {
            Self {
                src: s.as_bytes(),
                i: 0,
            }
        }
        const fn eof(&self) -> bool {
            self.i >= self.src.len()
        }
        fn peek(&self) -> Option<u8> {
            self.src.get(self.i).copied()
        }
        fn next(&mut self) -> Option<u8> {
            let c = self.peek();
            if c.is_some() {
                self.i += 1;
            }
            c
        }
        fn skip_ws(&mut self) {
            while let Some(b) = self.peek() {
                if (b as char).is_whitespace() {
                    self.i += 1;
                } else {
                    break;
                }
            }
        }
        fn parse_value(&mut self) -> Result<InputValue, AbiCodecError> {
            self.skip_ws();
            match self.peek() {
                Some(b'[') => self.parse_array(),
                Some(b'(') => self.parse_tuple(),
                Some(b'{') => self.parse_struct(),
                Some(b't') | Some(b'f') => self.parse_bool(),
                Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
                _ => Err(AbiCodecError::ParseError(format!(
                    "Unexpected character '{}' at position {}. Expected number, boolean, tuple '(' or '[', or struct '{{'",
                    self.peek().map(|b| b as char).unwrap_or('\0'),
                    self.i
                ))),
            }
        }
        fn parse_array(&mut self) -> Result<InputValue, AbiCodecError> {
            self.parse_delimited_list(b'[', b']', "array")
                .map(InputValue::List)
        }
        fn parse_tuple(&mut self) -> Result<InputValue, AbiCodecError> {
            self.parse_delimited_list(b'(', b')', "tuple")
                .map(InputValue::List)
        }
        fn parse_struct(&mut self) -> Result<InputValue, AbiCodecError> {
            self.parse_delimited_list(b'{', b'}', "struct")
                .map(InputValue::Struct)
        }

        fn parse_delimited_list(
            &mut self,
            open: u8,
            close: u8,
            context: &str,
        ) -> Result<Vec<InputValue>, AbiCodecError> {
            assert_eq!(self.next(), Some(open));
            self.skip_ws();
            let mut items = Vec::new();
            if self.peek() == Some(close) {
                self.next();
                return Ok(items);
            }
            loop {
                items.push(self.parse_value()?);
                self.skip_ws();
                match self.peek() {
                    Some(b',') => {
                        self.next();
                        self.skip_ws();
                    }
                    Some(c) if c == close => {
                        self.next();
                        break;
                    }
                    _ => {
                        return Err(AbiCodecError::ParseError(format!(
                            "Expected ',' or '{}' at position {} in {}",
                            close as char, self.i, context
                        )))
                    }
                }
            }
            Ok(items)
        }
        fn parse_bool(&mut self) -> Result<InputValue, AbiCodecError> {
            if self
                .src
                .get(self.i..self.i + 4)
                .map(|s| s == b"true")
                .unwrap_or(false)
            {
                self.i += 4;
                Ok(InputValue::Bool(true))
            } else if self
                .src
                .get(self.i..self.i + 5)
                .map(|s| s == b"false")
                .unwrap_or(false)
            {
                self.i += 5;
                Ok(InputValue::Bool(false))
            } else {
                Err(AbiCodecError::ParseError(format!(
                    "Invalid boolean at position {}: expected 'true' or 'false'",
                    self.i
                )))
            }
        }
        fn parse_number(&mut self) -> Result<InputValue, AbiCodecError> {
            let start = self.i;
            if self.peek() == Some(b'-') {
                self.i += 1;
            }
            let mut seen_digit = false;
            while let Some(b'0'..=b'9') = self.peek() {
                seen_digit = true;
                self.i += 1;
            }
            if !seen_digit {
                return Err(AbiCodecError::ParseError(format!(
                    "Invalid number starting at position {}",
                    start
                )));
            }
            let s = std::str::from_utf8(&self.src[start..self.i]).unwrap();
            let n = s.parse::<i64>().map_err(|e| {
                AbiCodecError::ParseError(format!("Failed to parse number '{}': {}", s, e))
            })?;
            Ok(InputValue::Number(n))
        }
    }

    let mut p = Parser::new(s);
    let v = p.parse_value()?;
    p.skip_ws();
    if !p.eof() {
        Err(AbiCodecError::ParseError(format!(
            "Unexpected trailing characters at position {}",
            p.i
        )))
    } else {
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json as json;

    // ==================== Unit Tests ====================
    // Tests for specific edge cases and validation logic not covered by proptest

    mod edge_cases {
        use super::*;

        #[test]
        fn test_m31_from_i64_extremes() {
            // Test i64::MIN and i64::MAX
            let min_felt = m31_from_i64(i64::MIN);
            let max_felt = m31_from_i64(i64::MAX);
            assert!(min_felt.0 < P);
            assert!(max_felt.0 < P);

            // Test modular arithmetic
            assert_eq!(m31_from_i64(-1), M31::reduce((P as u64) - 1));
            assert_eq!(m31_from_i64(-(P as i64)), M31::from(0u32));
            assert_eq!(m31_from_i64(-((P as i64) + 5)), M31::reduce((P as u64) - 5));
        }
    }

    // ==================== Parser Tests ====================
    // Parametrized tests for CLI argument parsing

    mod parser {
        use super::*;

        // Macro for concise test definitions
        macro_rules! parse_test {
            ($name:ident, $input:expr => Ok($expected:expr)) => {
                #[test]
                fn $name() {
                    assert_eq!(parse_cli_arg($input).unwrap(), $expected);
                }
            };
            ($name:ident, $input:expr => Err($substr:expr)) => {
                #[test]
                fn $name() {
                    let err = parse_cli_arg($input).unwrap_err().to_string();
                    assert!(
                        err.contains($substr),
                        "Expected error containing '{}', got: {}",
                        $substr,
                        err
                    );
                }
            };
        }

        // Success cases
        parse_test!(empty_struct, "{}" => Ok(InputValue::Struct(vec![])));
        parse_test!(empty_tuple, "[]" => Ok(InputValue::List(vec![])));
        parse_test!(empty_parens, "()" => Ok(InputValue::List(vec![])));

        parse_test!(simple_number, "42" => Ok(42.into()));
        parse_test!(negative_number, "-5" => Ok(InputValue::Number(-5)));
        parse_test!(bool_true, "true" => Ok(InputValue::Bool(true)));
        parse_test!(bool_false, "false" => Ok(InputValue::Bool(false)));

        parse_test!(nested_structures, "[1,{2,[3,4],(5,6)},false]" => Ok(InputValue::List(vec![
            1.into(),
            InputValue::Struct(vec![
                2.into(),
                InputValue::List(vec![3.into(), 4.into()]),
                InputValue::List(vec![5.into(), 6.into()])
            ]),
            InputValue::Bool(false)
        ])));

        parse_test!(whitespace_handling, "  { 1 , 2 , [ 3 , 4 ] }  " => Ok(InputValue::Struct(vec![
            1.into(),
            2.into(),
            InputValue::List(vec![3.into(), 4.into()])
        ])));

        // Error cases
        parse_test!(empty_input, "" => Err("Expected"));
        parse_test!(incomplete_struct, "{1," => Err("Unexpected character"));
        parse_test!(incomplete_array, "[1,2," => Err("Unexpected character"));
        parse_test!(invalid_bool, "tru" => Err("Invalid boolean"));
        parse_test!(trailing_chars, "{1} extra" => Err("trailing"));
    }

    // ==================== Integration Tests ====================
    // Tests for complex scenarios and JSON serialization

    mod integration {
        use crate::program::AbiType;

        use super::*;

        #[test]
        fn abi_type_json_roundtrip() {
            let ty = AbiType::Struct {
                name: "ComplexStruct".into(),
                fields: vec![
                    ("a".into(), AbiType::Felt),
                    (
                        "b".into(),
                        AbiType::FixedSizeArray {
                            element: Box::new(AbiType::U32),
                            size: 2,
                        },
                    ),
                    ("c".into(), AbiType::Tuple(vec![AbiType::Bool])),
                ],
            };
            let s = json::to_string(&ty).unwrap();
            let de: AbiType = json::from_str(&s).unwrap();
            assert_eq!(ty, de);
        }
    }
}
