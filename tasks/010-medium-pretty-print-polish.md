# 010-medium-pretty-print-polish.md

**Priority:** MEDIUM  
**Dependencies:** Task 001 (requires aggregate instructions)

## Why

Readable debug output is crucial for developer productivity when working with
the new aggregate operations. The MIR pretty-printing system serves multiple
critical purposes:

1. **Developer debugging**: Clear, readable MIR output helps developers
   understand how their code is being transformed and optimized
2. **Snapshot testing**: The compiler's extensive snapshot test suite relies on
   consistent, human-readable MIR formatting to catch regressions
3. **Optimization verification**: When implementing and debugging optimization
   passes, readable MIR output makes it easier to verify correctness
4. **Documentation and education**: Well-formatted MIR serves as documentation
   for how aggregate operations work in the compiler

The new aggregate instructions (`MakeTuple`, `ExtractTupleElement`,
`MakeStruct`, `ExtractStructField`) need consistent formatting that matches the
existing MIR pretty-printing style to maintain the developer experience and
testing infrastructure.

## What

Implement pretty-printing support for the new first-class aggregate instructions
with the following format specifications:

### Tuple Operations

```mir
// Tuple creation with elements %t = maketuple %0, %1, %2

// Tuple element extraction with index %v = extracttuple %t, 1

// Empty tuple %empty = maketuple
```

### Struct Operations

```mir
// Struct creation with named fields
%s = makestruct { x: %0, y: %1, z: %2 }

// Struct field extraction with field name
%v = extractfield %s, "x"

// Single field struct
%point = makestruct { value: %42 }

// Empty struct (if supported)
%empty = makestruct { }
```

### Optional: Struct Field Insertion

```mir
// Create new struct with updated field %s1 = insertfield %s0, "x", %new_x
```

The formatting follows these principles:

- Consistent with existing MIR operations (using `=` assignment syntax)
- Clear separation between operation name and operands
- Structured field syntax for structs using `{ field: value, ... }`
- Simple comma-separated syntax for tuples
- Type information included only when necessary for disambiguation

## How

### 1. Update PrettyPrint Implementation

**File:** `crates/compiler/mir/src/instruction.rs`

Add new cases to the `impl PrettyPrint for Instruction` match statement:

```rust
// In the match &self.kind block, add:

InstructionKind::MakeTuple { dest, elements } => {
    let elements_str = elements
        .iter()
        .map(|val| val.pretty_print(0))
        .collect::<Vec<_>>()
        .join(", ");
    result.push_str(&format!(
        "{} = maketuple {}",
        dest.pretty_print(0),
        elements_str
    ));
}

InstructionKind::ExtractTupleElement { dest, tuple, index, .. } => {
    result.push_str(&format!(
        "{} = extracttuple {}, {}",
        dest.pretty_print(0),
        tuple.pretty_print(0),
        index
    ));
}

InstructionKind::MakeStruct { dest, fields, .. } => {
    let fields_str = fields
        .iter()
        .map(|(name, val)| format!("{}: {}", name, val.pretty_print(0)))
        .collect::<Vec<_>>()
        .join(", ");
    result.push_str(&format!(
        "{} = makestruct {{ {} }}",
        dest.pretty_print(0),
        fields_str
    ));
}

InstructionKind::ExtractStructField { dest, struct_val, field_name, .. } => {
    result.push_str(&format!(
        "{} = extractfield {}, \"{}\"",
        dest.pretty_print(0),
        struct_val.pretty_print(0),
        field_name
    ));
}

// Optional: if InsertField is implemented
InstructionKind::InsertField { dest, struct_val, field_name, value, .. } => {
    result.push_str(&format!(
        "{} = insertfield {}, \"{}\", {}",
        dest.pretty_print(0),
        struct_val.pretty_print(0),
        field_name,
        value.pretty_print(0)
    ));
}
```

### 2. Format Specifications for Each Instruction

#### MakeTuple Format

- **Syntax:** `%dest = maketuple %elem1, %elem2, ...`
- **Empty tuple:** `%dest = maketuple`
- **Single element:** `%dest = maketuple %elem`
- **Preserves element order as specified in the source**

#### ExtractTupleElement Format

- **Syntax:** `%dest = extracttuple %tuple, index`
- **Index is zero-based integer literal**
- **No type annotation unless required for disambiguation**

#### MakeStruct Format

- **Syntax:** `%dest = makestruct { field1: %val1, field2: %val2 }`
- **Field names are identifiers (no quotes unless necessary)**
- **Empty struct:** `%dest = makestruct { }`
- **Fields are ordered as they appear in the source or struct definition**

#### ExtractStructField Format

- **Syntax:** `%dest = extractfield %struct, "fieldname"`
- **Field names are quoted strings for clarity and to handle special
  characters**
- **Consistent with GEP field access patterns in existing MIR**

#### InsertField Format (Optional)

- **Syntax:** `%dest = insertfield %struct, "fieldname", %newval`
- **Creates new struct value with updated field**
- **Original struct remains unchanged (functional update)**

### 3. Snapshot Test Updates

**File:** `crates/compiler/mir/src/instruction_tests.rs` (create if needed)

Add unit tests that verify pretty-printing format:

```rust
#[cfg(test)]
mod pretty_print_tests {
    use super::*;
    use crate::{MirFunction, MirType, Value};

    #[test]
    fn test_make_tuple_pretty_print() {
        let mut func = MirFunction::new("test".to_string());
        let dest = func.new_value_id();
        let elem1 = func.new_value_id();
        let elem2 = func.new_value_id();

        let instr = Instruction::make_tuple(
            dest,
            vec![Value::operand(elem1), Value::operand(elem2)]
        );

        let output = instr.pretty_print(0);
        insta::assert_snapshot!(output, @"%2 = maketuple %0, %1");
    }

    #[test]
    fn test_make_struct_pretty_print() {
        let mut func = MirFunction::new("test".to_string());
        let dest = func.new_value_id();
        let x_val = func.new_value_id();
        let y_val = func.new_value_id();

        let instr = Instruction::make_struct(
            dest,
            vec![
                ("x".to_string(), Value::operand(x_val)),
                ("y".to_string(), Value::operand(y_val))
            ],
            MirType::Unknown
        );

        let output = instr.pretty_print(0);
        insta::assert_snapshot!(output, @"%2 = makestruct { x: %0, y: %1 }");
    }

    #[test]
    fn test_extract_operations_pretty_print() {
        let mut func = MirFunction::new("test".to_string());
        let tuple_dest = func.new_value_id();
        let tuple_val = func.new_value_id();
        let struct_dest = func.new_value_id();
        let struct_val = func.new_value_id();

        let tuple_extract = Instruction::extract_tuple_element(
            tuple_dest,
            Value::operand(tuple_val),
            1,
            MirType::Felt
        );

        let struct_extract = Instruction::extract_struct_field(
            struct_dest,
            Value::operand(struct_val),
            "field_name".to_string(),
            MirType::Felt
        );

        insta::assert_snapshot!(tuple_extract.pretty_print(0), @"%1 = extracttuple %0, 1");
        insta::assert_snapshot!(struct_extract.pretty_print(0), @"%1 = extractfield %0, \"field_name\"");
    }
}
```

Update existing snapshot tests by running:

```bash
cargo test
cargo insta review  # Review changes to snapshot files
cargo insta accept  # Accept new format if correct
```

### 4. Debug Output Examples

#### Before (Memory-Based):

```mir
fn example {
  entry: 0

  0:
    %0 = framealloc (felt, felt)
    store %0, 0, 42
    store %0, 1, 24
    %1 = getelementptr %0, 0
    %2 = load %1
    return %2
}
```

#### After (Value-Based):

```mir
fn example {
  entry: 0

  0:
    %0 = maketuple 42, 24
    %1 = extracttuple %0, 0
    return %1
}
```

#### Struct Example:

```mir
fn point_example {
  entry: 0

  0:
    %0 = makestruct { x: 10, y: 20 }
    %1 = extractfield %0, "x"
    %2 = insertfield %0, "x", 15
    %3 = extractfield %2, "x"
    return %3
}
```

#### Complex Nested Example:

```mir
fn nested_example {
  entry: 0

  0:
    %0 = makestruct { x: 1, y: 2 }
    %1 = makestruct { x: 3, y: 4 }
    %2 = maketuple %0, %1
    %3 = extracttuple %2, 0
    %4 = extractfield %3, "x"
    return %4
}
```

### Implementation Steps Summary

1. **Add pretty-print cases** for each new instruction type in `instruction.rs`
2. **Write unit tests** with snapshot assertions to lock in the format
3. **Update documentation** to reflect the new MIR syntax
4. **Run full test suite** and update any affected snapshots
5. **Verify consistency** with existing MIR formatting conventions

### Success Criteria

- [ ] All new aggregate instructions have readable pretty-print output
- [ ] Format is consistent with existing MIR instruction formatting
- [ ] Snapshot tests capture and verify the format
- [ ] Full compiler test suite passes with updated snapshots
- [ ] Debug output clearly shows the transformation from memory-based to
      value-based aggregates
- [ ] Developers can easily read and understand MIR containing aggregate
      operations

This pretty-printing enhancement is essential for maintaining developer
productivity and testing infrastructure as we transition to the new
aggregate-first MIR design.
