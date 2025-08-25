use crate::program::{AbiSlot, AbiType};
use stwo_prover::core::fields::m31::{M31, P};

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
    #[error("Dynamic arrays not supported in ABI encoding/decoding")]
    DynamicArrayUnsupported,
    #[error("Unexpected trailing or insufficient return data")]
    TrailingOrInsufficientData,
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Encode input values according to ABI parameter types
pub fn encode_input_args(
    params: &[AbiSlot],
    values: &[InputValue],
) -> Result<Vec<M31>, AbiCodecError> {
    if params.len() != values.len() {
        return Err(AbiCodecError::TypeMismatch(format!(
            "arg count mismatch: expected {} got {}",
            params.len(),
            values.len()
        )));
    }
    let mut out = Vec::new();
    for (slot, val) in params.iter().zip(values.iter()) {
        encode_input(&mut out, &slot.ty, val)?;
    }
    Ok(out)
}

/// Decode memory values into typed CairoMValue based on ABI types
pub fn decode_abi_values(
    returns: &[AbiSlot],
    src: &[M31],
) -> Result<Vec<CairoMValue>, AbiCodecError> {
    let mut cursor = 0usize;
    let mut out = Vec::with_capacity(returns.len());
    for slot in returns {
        let (v, next) = decode_one(&slot.ty, src, cursor)?;
        cursor = next;
        out.push(v);
    }
    // Check for trailing data
    if cursor != src.len() {
        return Err(AbiCodecError::TrailingOrInsufficientData);
    }
    Ok(out)
}

/// Convert an i64 to an M31
const fn m31_from_i64(n: i64) -> M31 {
    // Perform modular reduction over Z, then map to M31 safely.
    // Works for all i64, including large magnitude negatives.
    let p = P as i128;
    let mut m = (n as i128) % p;
    if m < 0 {
        m += p;
    }
    M31::reduce(m as u64)
}

/// Encode an input value according to its ABI type (AbiType is the source of truth)
fn encode_input(dst: &mut Vec<M31>, ty: &AbiType, val: &InputValue) -> Result<(), AbiCodecError> {
    match (ty, val) {
        // Numbers can be interpreted based on the expected type
        (AbiType::Felt, InputValue::Number(n)) => {
            dst.push(m31_from_i64(*n));
            Ok(())
        }
        (AbiType::U32, InputValue::Number(n)) => {
            if *n < 0 || *n as i128 > u32::MAX as i128 {
                return Err(AbiCodecError::TypeMismatch(format!(
                    "u32 out of range: {}",
                    n
                )));
            }
            let u = *n as u32;
            let lo = M31::from(u & 0xFFFF);
            let hi = M31::from(u >> 16);
            dst.extend_from_slice(&[lo, hi]);
            Ok(())
        }
        (AbiType::Bool, InputValue::Number(n)) => {
            match *n {
                0 => dst.push(M31::from(0u32)),
                1 => dst.push(M31::from(1u32)),
                _ => {
                    return Err(AbiCodecError::TypeMismatch(format!(
                        "bool expects 0 or 1, got {}",
                        n
                    )))
                }
            }
            Ok(())
        }
        (AbiType::Bool, InputValue::Bool(b)) => {
            dst.push(M31::from(if *b { 1u32 } else { 0u32 }));
            Ok(())
        }
        (AbiType::Pointer(_), _) => {
            // Pointers are internal-only and cannot be provided as CLI arguments
            // They're only used for decoding values from program memory
            Err(AbiCodecError::TypeMismatch(
                "Pointer types are internal-only and cannot be provided as input.".to_string(),
            ))
        }

        // Collections
        (AbiType::Tuple(types), InputValue::List(values)) => {
            if types.len() != values.len() {
                return Err(AbiCodecError::TypeMismatch(format!(
                    "tuple arity mismatch: expected {} got {}",
                    types.len(),
                    values.len()
                )));
            }
            for (t, v) in types.iter().zip(values.iter()) {
                encode_input(dst, t, v)?;
            }
            Ok(())
        }
        (AbiType::Struct { fields, .. }, InputValue::Struct(values)) => {
            if fields.len() != values.len() {
                return Err(AbiCodecError::TypeMismatch(format!(
                    "struct field count mismatch: expected {} got {}",
                    fields.len(),
                    values.len()
                )));
            }
            // Positional encoding - field names come from AbiType
            for ((_, fty), v) in fields.iter().zip(values.iter()) {
                encode_input(dst, fty, v)?;
            }
            Ok(())
        }
        (AbiType::Array { .. }, InputValue::List(_)) => {
            // TODO: Array support requires parser implementation
            Err(AbiCodecError::TypeMismatch(
                "Array types are not yet supported in Cairo-M. Parser implementation pending."
                    .to_string(),
            ))
        }
        (AbiType::Unit, InputValue::Unit) => Ok(()),
        _ => {
            let err_str = format!("incompatible type/value pair: {:?}/{:?}", ty, val);
            Err(AbiCodecError::TypeMismatch(err_str))
        }
    }
}

fn decode_one(
    ty: &AbiType,
    src: &[M31],
    start: usize,
) -> Result<(CairoMValue, usize), AbiCodecError> {
    match ty {
        AbiType::Felt => {
            if start + 1 > src.len() {
                return Err(AbiCodecError::InsufficientData);
            }
            Ok((CairoMValue::Felt(src[start]), start + 1))
        }
        AbiType::Bool => {
            if start + 1 > src.len() {
                return Err(AbiCodecError::InsufficientData);
            }
            let val = src[start].0;
            // Validate boolean value is 0 or 1
            if val != 0 && val != 1 {
                return Err(AbiCodecError::TypeMismatch(format!(
                    "Invalid boolean value: expected 0 or 1, got {}",
                    val
                )));
            }
            Ok((CairoMValue::Bool(val == 1), start + 1))
        }
        AbiType::U32 => {
            if start + 2 > src.len() {
                return Err(AbiCodecError::InsufficientData);
            }
            let lo = src[start].0;
            let hi = src[start + 1].0;
            // Validate that each part fits in 16 bits
            if lo >= (1 << 16) || hi >= (1 << 16) {
                return Err(AbiCodecError::TypeMismatch(format!(
                    "Invalid U32 value: lo={}, hi={} (each must be < 65536)",
                    lo, hi
                )));
            }
            Ok((CairoMValue::U32(lo | (hi << 16)), start + 2))
        }
        AbiType::Pointer(_) => {
            if start + 1 > src.len() {
                return Err(AbiCodecError::InsufficientData);
            }
            Ok((CairoMValue::Pointer(src[start]), start + 1))
        }
        AbiType::Tuple(elems) => {
            let mut cur = start;
            let mut out = Vec::with_capacity(elems.len());
            for t in elems {
                let (v, next) = decode_one(t, src, cur)?;
                cur = next;
                out.push(v);
            }
            Ok((CairoMValue::Tuple(out), cur))
        }
        AbiType::Struct { fields, .. } => {
            let mut cur = start;
            let mut out = Vec::with_capacity(fields.len());
            for (fname, fty) in fields {
                let (v, next) = decode_one(fty, src, cur)?;
                cur = next;
                out.push((fname.clone(), v));
            }
            Ok((CairoMValue::Struct(out), cur))
        }
        AbiType::Array { element, size } => {
            let n = size.ok_or(AbiCodecError::DynamicArrayUnsupported)? as usize;
            let mut cur = start;
            let mut out = Vec::with_capacity(n);
            for _ in 0..n {
                let (v, next) = decode_one(element, src, cur)?;
                cur = next;
                out.push(v);
            }
            Ok((CairoMValue::Array(out), cur))
        }
        AbiType::Unit => Ok((CairoMValue::Unit, start)),
    }
}

/// Parse a CLI argument string into an InputValue (supports nesting)
///
/// Supported grammar (positional structs):
///   Value := Number | Bool | Array | Struct
///   Array := '[' (Value (',' Value)*)? ']'
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
            assert_eq!(self.next(), Some(b'['));
            self.skip_ws();
            let mut items = Vec::new();
            if self.peek() == Some(b']') {
                self.next();
                return Ok(InputValue::List(items));
            }
            loop {
                items.push(self.parse_value()?);
                self.skip_ws();
                match self.peek() {
                    Some(b',') => {
                        self.next();
                        self.skip_ws();
                    }
                    Some(b']') => {
                        self.next();
                        break;
                    }
                    _ => {
                        return Err(AbiCodecError::ParseError(format!(
                            "Expected ',' or ']' at position {} in tuple",
                            self.i
                        )))
                    }
                }
            }
            Ok(InputValue::List(items))
        }
        fn parse_tuple(&mut self) -> Result<InputValue, AbiCodecError> {
            assert_eq!(self.next(), Some(b'('));
            self.skip_ws();
            let mut items = Vec::new();
            if self.peek() == Some(b')') {
                self.next();
                return Ok(InputValue::List(items));
            }
            loop {
                items.push(self.parse_value()?);
                self.skip_ws();
                match self.peek() {
                    Some(b',') => {
                        self.next();
                        self.skip_ws();
                    }
                    Some(b')') => {
                        self.next();
                        break;
                    }
                    _ => {
                        return Err(AbiCodecError::ParseError(format!(
                            "Expected ',' or ')' at position {} in tuple",
                            self.i
                        )))
                    }
                }
            }
            Ok(InputValue::List(items))
        }
        fn parse_struct(&mut self) -> Result<InputValue, AbiCodecError> {
            assert_eq!(self.next(), Some(b'{'));
            self.skip_ws();
            let mut fields = Vec::new();
            if self.peek() == Some(b'}') {
                self.next();
                return Ok(InputValue::Struct(fields));
            }
            loop {
                fields.push(self.parse_value()?);
                self.skip_ws();
                match self.peek() {
                    Some(b',') => {
                        self.next();
                        self.skip_ws();
                    }
                    Some(b'}') => {
                        self.next();
                        break;
                    }
                    _ => {
                        return Err(AbiCodecError::ParseError(format!(
                            "Expected ',' or '}}' at position {} in struct",
                            self.i
                        )))
                    }
                }
            }
            Ok(InputValue::Struct(fields))
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
    use proptest::prelude::*;
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

        #[test]
        fn test_invalid_bool_decode() {
            // Test decoding invalid boolean value (not 0 or 1)
            let src = vec![M31::from(42u32)];
            let result = decode_one(&AbiType::Bool, &src, 0);
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Invalid boolean value"));
        }

        #[test]
        fn test_invalid_u32_decode() {
            // Test decoding invalid U32 with values > 16 bits
            let src = vec![M31::from(70000u32), M31::from(0u32)]; // lo > 65535
            let result = decode_one(&AbiType::U32, &src, 0);
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Invalid U32 value"));
        }

        #[test]
        fn test_arrays_not_supported() {
            let slot = AbiSlot {
                name: "arr".into(),
                ty: AbiType::Array {
                    element: Box::new(AbiType::U32),
                    size: Some(3),
                },
            };
            let input = InputValue::List(vec![1.into(), 2.into(), 3.into()]);
            let result = encode_input_args(&[slot], &[input]);
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Array types are not yet supported"));
        }

        #[test]
        fn test_trailing_data_detection() {
            let slots = vec![AbiSlot {
                name: "x".into(),
                ty: AbiType::Felt,
            }];
            let src = vec![M31::from(42u32), M31::from(99u32)]; // Extra data
            let result = decode_abi_values(&slots, &src);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                AbiCodecError::TrailingOrInsufficientData
            ));
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
        use super::*;

        #[test]
        fn abi_type_json_roundtrip() {
            let ty = AbiType::Struct {
                name: "ComplexStruct".into(),
                fields: vec![
                    ("a".into(), AbiType::Felt),
                    (
                        "b".into(),
                        AbiType::Array {
                            element: Box::new(AbiType::U32),
                            size: Some(2),
                        },
                    ),
                    (
                        "c".into(),
                        AbiType::Tuple(vec![
                            AbiType::Bool,
                            AbiType::Pointer(Box::new(AbiType::Unit)),
                        ]),
                    ),
                ],
            };
            let s = json::to_string(&ty).unwrap();
            let de: AbiType = json::from_str(&s).unwrap();
            assert_eq!(ty, de);
        }

        #[test]
        fn complex_struct_roundtrip() {
            let ty = AbiType::Struct {
                name: "TestStruct".into(),
                fields: vec![
                    ("x".into(), AbiType::Felt),
                    (
                        "y".into(),
                        AbiType::Tuple(vec![AbiType::Bool, AbiType::U32]),
                    ),
                ],
            };
            let slot = AbiSlot {
                name: "s".into(),
                ty,
            };
            let input = InputValue::Struct(vec![
                InputValue::Number(-5),
                InputValue::List(vec![InputValue::Bool(true), 7.into()]),
            ]);

            // Encode
            let enc = encode_input_args(&[slot.clone()], &[input]).unwrap();
            assert_eq!(enc.len(), 4); // felt(1) + bool(1) + u32(2) = 4 slots

            // Decode and verify
            let vals = decode_abi_values(&[slot], &enc).unwrap();
            match &vals[0] {
                CairoMValue::Struct(fields) => {
                    assert_eq!(fields.len(), 2);
                    assert_eq!(fields[0].0, "x");
                    assert!(matches!(fields[1].1, CairoMValue::Tuple(_)));
                }
                other => panic!("unexpected value: {:?}", other),
            }
        }
    }

    // ==================== Property-Based Tests ====================
    // Comprehensive round-trip testing for all supported types

    #[cfg(test)]
    mod proptest_tests {
        use super::*;

        // Strategy for generating InputValue that matches a given AbiType
        fn arb_input_for_type(ty: &AbiType, depth: u32) -> BoxedStrategy<InputValue> {
            // Limit recursion depth
            if depth > 3 {
                return Just(InputValue::Unit).boxed();
            }

            match ty {
                AbiType::Felt => (-1_000_000i64..1_000_000)
                    .prop_map(InputValue::Number)
                    .boxed(),
                AbiType::Bool => prop_oneof![
                    Just(InputValue::Bool(true)),
                    Just(InputValue::Bool(false)),
                    Just(0.into()),
                    Just(1.into()),
                ]
                .boxed(),
                AbiType::U32 => (0u32..=u32::MAX)
                    .prop_map(|n| InputValue::Number(n as i64))
                    .boxed(),
                AbiType::Tuple(types) => {
                    let strategies: Vec<_> = types
                        .iter()
                        .map(|t| arb_input_for_type(t, depth + 1))
                        .collect();
                    strategies.prop_map(InputValue::List).boxed()
                }
                AbiType::Struct { fields, .. } => {
                    let strategies: Vec<_> = fields
                        .iter()
                        .map(|(_, t)| arb_input_for_type(t, depth + 1))
                        .collect();
                    strategies.prop_map(InputValue::Struct).boxed()
                }
                AbiType::Unit => Just(InputValue::Unit).boxed(),
                _ => Just(InputValue::Unit).boxed(), // Unsupported types
            }
        }

        // Strategy for generating simple AbiType instances (no arrays/pointers)
        fn arb_simple_abi_type(depth: u32) -> BoxedStrategy<AbiType> {
            if depth > 2 {
                return prop_oneof![
                    Just(AbiType::Felt),
                    Just(AbiType::Bool),
                    Just(AbiType::U32),
                    Just(AbiType::Unit),
                ]
                .boxed();
            }

            prop_oneof![
                Just(AbiType::Felt),
                Just(AbiType::Bool),
                Just(AbiType::U32),
                Just(AbiType::Unit),
                // Tuples with 0-3 elements
                prop::collection::vec(arb_simple_abi_type(depth + 1), 0..=3)
                    .prop_map(AbiType::Tuple),
                // Structs with 0-3 fields
                prop::collection::vec(("[a-z]+", arb_simple_abi_type(depth + 1)), 0..=3).prop_map(
                    |fields| AbiType::Struct {
                        name: "TestStruct".to_string(),
                        fields,
                    }
                ),
            ]
            .boxed()
        }

        proptest! {
            #[test]
            fn felt_roundtrip(n in 0i64..=i64::MAX) {
                let slot = AbiSlot {
                    name: "test".to_string(),
                    ty: AbiType::Felt,
                };
                let input = InputValue::Number(n);

                let encoded = encode_input_args(&[slot.clone()], &[input]).unwrap();
                let decoded = decode_abi_values(&[slot], &encoded).unwrap();

                assert_eq!(decoded.len(), 1);
                match &decoded[0] {
                    CairoMValue::Felt(m) => {
                        assert_eq!(*m, m31_from_i64(n));
                    }
                    _ => panic!("Expected Felt, got {:?}", decoded[0]),
                }
            }

            #[test]
            fn bool_roundtrip(b in any::<bool>()) {
                let slot = AbiSlot {
                    name: "test".to_string(),
                    ty: AbiType::Bool,
                };
                let input = InputValue::Bool(b);

                let encoded = encode_input_args(&[slot.clone()], &[input]).unwrap();
                let decoded = decode_abi_values(&[slot], &encoded).unwrap();

                assert_eq!(decoded.len(), 1);
                assert_eq!(decoded[0], CairoMValue::Bool(b));
            }

            #[test]
            fn u32_roundtrip(n in 0u32..=u32::MAX) {
                let slot = AbiSlot {
                    name: "test".to_string(),
                    ty: AbiType::U32,
                };
                let input = InputValue::Number(n as i64);

                let encoded = encode_input_args(&[slot.clone()], &[input]).unwrap();
                let decoded = decode_abi_values(&[slot], &encoded).unwrap();

                assert_eq!(decoded.len(), 1);
                assert_eq!(decoded[0], CairoMValue::U32(n));
            }

            #[test]
            fn u32_validation(n in i64::MIN..=i64::MAX) {
                let slot = AbiSlot {
                    name: "test".to_string(),
                    ty: AbiType::U32,
                };
                let input = InputValue::Number(n);

                let result = encode_input_args(&[slot], &[input]);
                if n >= 0 && n <= u32::MAX as i64 {
                    prop_assert!(result.is_ok());
                } else {
                    prop_assert!(result.is_err());
                }
            }

            #[test]
            fn bool_validation(n in i64::MIN..=i64::MAX) {
                let slot = AbiSlot {
                    name: "test".to_string(),
                    ty: AbiType::Bool,
                };
                let input = InputValue::Number(n);

                let result = encode_input_args(&[slot], &[input]);
                if n == 0 || n == 1 {
                    prop_assert!(result.is_ok());
                } else {
                    prop_assert!(result.is_err());
                }
            }

            #[test]
            fn complex_type_roundtrip(ty in arb_simple_abi_type(0)) {
                let slot = AbiSlot {
                    name: "test".to_string(),
                    ty: ty.clone(),
                };

                // Generate matching input
                let input_strategy = arb_input_for_type(&ty, 0);

                proptest::prop_assume!(matches!(ty,
                    AbiType::Felt | AbiType::Bool | AbiType::U32 |
                    AbiType::Unit | AbiType::Tuple(_) | AbiType::Struct { .. }
                ));

                // Generate a valid input for this type
                let mut runner = proptest::test_runner::TestRunner::deterministic();
                let input = input_strategy.new_tree(&mut runner).unwrap().current();

                // Try to encode and decode
                if let Ok(encoded) = encode_input_args(&[slot.clone()], &[input]) {
                    let decoded = decode_abi_values(&[slot], &encoded);
                    prop_assert!(decoded.is_ok(), "Decode failed after successful encode");
                    prop_assert_eq!(decoded.unwrap().len(), 1);
                }
                else {
                    prop_assert!(false, "Encode failed");
                }
            }
        }
    }
}
