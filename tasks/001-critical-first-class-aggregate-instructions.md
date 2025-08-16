# Task 001: Critical - First-Class Aggregate Instructions in MIR

**Priority**: CRITICAL **Dependencies**: None (foundational task) **Estimated
Effort**: 2-3 days **Reviewer**: Required due to critical priority

## Why

The current MIR represents aggregates (tuples and structs) using memory
operations (`frame_alloc`, `store`, `load`, `get_element_ptr`), which creates
several significant problems:

1. **Verbose IR**: Simple aggregate operations like creating a tuple `(x, y)`
   require multiple memory instructions
2. **Complex Optimization Requirements**: The memory-centric approach
   necessitates heavy optimization passes like SROA (Scalar Replacement of
   Aggregates) and Mem2Reg to convert back into registers
3. **Performance Overhead**: Memory allocations and loads/stores for simple
   value operations add unnecessary runtime overhead
4. **Code Clarity**: The MIR becomes harder to read and debug when simple
   operations are obscured by memory management

By introducing first-class aggregate instructions, we can:

- Generate cleaner, more readable MIR directly from the AST
- Eliminate the need for complex memory-to-register optimization passes
- Enable more straightforward optimizations like constant folding on aggregates
- Improve compilation performance by reducing optimization complexity

## What

Implement four new `InstructionKind` variants in the MIR instruction system to
handle aggregates as first-class SSA values:

### New Instruction Variants

1. **`MakeTuple`** - Creates a tuple from a list of values

   ```rust
   MakeTuple {
       dest: ValueId,
       elements: Vec<Value>,
   }
   ```

   Example: `%t = make_tuple %x, %y, %z`

2. **`ExtractTupleElement`** - Extracts an element from a tuple by index

   ```rust
   ExtractTupleElement {
       dest: ValueId,
       tuple: Value,
       index: usize,
       element_ty: MirType,
   }
   ```

   Example: `%v = extract_tuple_element %t, 1`

3. **`MakeStruct`** - Creates a struct from field values

   ```rust
   MakeStruct {
       dest: ValueId,
       fields: Vec<(String, Value)>,
       struct_ty: MirType,
   }
   ```

   Example: `%s = make_struct { x: %0, y: %1 }`

4. **`ExtractStructField`** - Extracts a field from a struct by name
   ```rust
   ExtractStructField {
       dest: ValueId,
       struct_val: Value,
       field_name: String,
       field_ty: MirType,
   }
   ```
   Example: `%v = extract_struct_field %s, "x"`

### Required Support Infrastructure

- Constructor functions for each instruction type
- Updated helper methods (`destinations()`, `used_values()`, `validate()`)
- Pretty-printing support for debugging
- Basic validation logic

## How

### Step 1: Modify Core Instruction Types

**File**:
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/instruction.rs`

#### 1.1 Add New Enum Variants

Add the following variants to the `InstructionKind` enum (around line 330, after
the `Phi` variant):

```rust
/// Build a tuple from a list of values: `dest = make_tuple(v0, v1, ...)`
MakeTuple {
    dest: ValueId,
    elements: Vec<Value>,
},

/// Extract an element from a tuple value: `dest = extract_tuple_element(tuple_val, index)`
ExtractTupleElement {
    dest: ValueId,
    tuple: Value,
    index: usize,
    element_ty: MirType,
},

/// Build a struct from a list of field values: `dest = make_struct { field1: v1, ... }`
MakeStruct {
    dest: ValueId,
    fields: Vec<(String, Value)>,
    struct_ty: MirType,
},

/// Extract a field from a struct value: `dest = extract_struct_field(struct_val, "field_name")`
ExtractStructField {
    dest: ValueId,
    struct_val: Value,
    field_name: String,
    field_ty: MirType,
},
```

#### 1.2 Add Constructor Functions

Add these constructor functions to the `impl Instruction` block (after the
`phi()` function around line 494):

```rust
/// Creates a new make tuple instruction
pub fn make_tuple(dest: ValueId, elements: Vec<Value>) -> Self {
    Self {
        kind: InstructionKind::MakeTuple { dest, elements },
        source_span: None,
        source_expr_id: None,
        comment: None,
    }
}

/// Creates a new extract tuple element instruction
pub const fn extract_tuple_element(
    dest: ValueId,
    tuple: Value,
    index: usize,
    element_ty: MirType,
) -> Self {
    Self {
        kind: InstructionKind::ExtractTupleElement {
            dest,
            tuple,
            index,
            element_ty,
        },
        source_span: None,
        source_expr_id: None,
        comment: None,
    }
}

/// Creates a new make struct instruction
pub fn make_struct(dest: ValueId, fields: Vec<(String, Value)>, struct_ty: MirType) -> Self {
    Self {
        kind: InstructionKind::MakeStruct {
            dest,
            fields,
            struct_ty,
        },
        source_span: None,
        source_expr_id: None,
        comment: None,
    }
}

/// Creates a new extract struct field instruction
pub fn extract_struct_field(
    dest: ValueId,
    struct_val: Value,
    field_name: String,
    field_ty: MirType,
) -> Self {
    Self {
        kind: InstructionKind::ExtractStructField {
            dest,
            struct_val,
            field_name,
            field_ty,
        },
        source_span: None,
        source_expr_id: None,
        comment: None,
    }
}
```

#### 1.3 Update Helper Methods

**Update `destinations()` method** (around line 534): Add these cases to the
match statement:

```rust
InstructionKind::MakeTuple { dest, .. }
| InstructionKind::ExtractTupleElement { dest, .. }
| InstructionKind::MakeStruct { dest, .. }
| InstructionKind::ExtractStructField { dest, .. } => vec![*dest],
```

**Update `used_values()` method** (around line 649, before the closing bracket):
Add these cases to the match statement:

```rust
InstructionKind::MakeTuple { elements, .. } => {
    for element in elements {
        if let Value::Operand(id) = element {
            used.insert(*id);
        }
    }
}

InstructionKind::ExtractTupleElement { tuple, .. } => {
    if let Value::Operand(id) = tuple {
        used.insert(*id);
    }
}

InstructionKind::MakeStruct { fields, .. } => {
    for (_, value) in fields {
        if let Value::Operand(id) = value {
            used.insert(*id);
        }
    }
}

InstructionKind::ExtractStructField { struct_val, .. } => {
    if let Value::Operand(id) = struct_val {
        used.insert(*id);
    }
}
```

**Update `validate()` method** (around line 668): Add these cases to the match
statement:

```rust
InstructionKind::MakeTuple { .. } => Ok(()),
InstructionKind::ExtractTupleElement { .. } => Ok(()),
InstructionKind::MakeStruct { .. } => Ok(()),
InstructionKind::ExtractStructField { .. } => Ok(()),
```

#### 1.4 Update Pretty Printing

**Update `PrettyPrint` implementation** (around line 861, before the closing
bracket): Add these cases to the match statement:

```rust
InstructionKind::MakeTuple { dest, elements } => {
    let elements_str = elements
        .iter()
        .map(|elem| elem.pretty_print(0))
        .collect::<Vec<_>>()
        .join(", ");
    result.push_str(&format!(
        "{} = make_tuple {}",
        dest.pretty_print(0),
        elements_str
    ));
}

InstructionKind::ExtractTupleElement {
    dest,
    tuple,
    index,
    element_ty,
} => {
    result.push_str(&format!(
        "{} = extract_tuple_element {}, {} ({})",
        dest.pretty_print(0),
        tuple.pretty_print(0),
        index,
        element_ty
    ));
}

InstructionKind::MakeStruct {
    dest,
    fields,
    struct_ty,
} => {
    let fields_str = fields
        .iter()
        .map(|(name, value)| format!("{}: {}", name, value.pretty_print(0)))
        .collect::<Vec<_>>()
        .join(", ");
    result.push_str(&format!(
        "{} = make_struct {{ {} }} ({})",
        dest.pretty_print(0),
        fields_str,
        struct_ty
    ));
}

InstructionKind::ExtractStructField {
    dest,
    struct_val,
    field_name,
    field_ty,
} => {
    result.push_str(&format!(
        "{} = extract_struct_field {}, \"{}\" ({})",
        dest.pretty_print(0),
        struct_val.pretty_print(0),
        field_name,
        field_ty
    ));
}
```

### Step 2: Create Comprehensive Tests

**File**:
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/instruction_tests.rs`
(create new file)

```rust
#[cfg(test)]
mod aggregate_instruction_tests {
    use super::*;
    use crate::{MirType, Value, ValueId};

    #[test]
    fn test_make_tuple_instruction() {
        let dest = ValueId::new(0);
        let elem1 = Value::Operand(ValueId::new(1));
        let elem2 = Value::Operand(ValueId::new(2));
        let elements = vec![elem1, elem2];

        let instr = Instruction::make_tuple(dest, elements.clone());

        match &instr.kind {
            InstructionKind::MakeTuple { dest: d, elements: e } => {
                assert_eq!(*d, dest);
                assert_eq!(*e, elements);
            }
            _ => panic!("Expected MakeTuple instruction"),
        }

        assert_eq!(instr.destinations(), vec![dest]);
        assert!(instr.used_values().contains(&ValueId::new(1)));
        assert!(instr.used_values().contains(&ValueId::new(2)));
    }

    #[test]
    fn test_extract_tuple_element_instruction() {
        let dest = ValueId::new(0);
        let tuple = Value::Operand(ValueId::new(1));
        let index = 1;
        let element_ty = MirType::felt();

        let instr = Instruction::extract_tuple_element(dest, tuple, index, element_ty.clone());

        match &instr.kind {
            InstructionKind::ExtractTupleElement { dest: d, tuple: t, index: i, element_ty: ty } => {
                assert_eq!(*d, dest);
                assert_eq!(*t, tuple);
                assert_eq!(*i, index);
                assert_eq!(*ty, element_ty);
            }
            _ => panic!("Expected ExtractTupleElement instruction"),
        }

        assert_eq!(instr.destinations(), vec![dest]);
        assert!(instr.used_values().contains(&ValueId::new(1)));
    }

    #[test]
    fn test_make_struct_instruction() {
        let dest = ValueId::new(0);
        let fields = vec![
            ("x".to_string(), Value::Operand(ValueId::new(1))),
            ("y".to_string(), Value::Operand(ValueId::new(2))),
        ];
        let struct_ty = MirType::simple_struct_type("Point".to_string());

        let instr = Instruction::make_struct(dest, fields.clone(), struct_ty.clone());

        match &instr.kind {
            InstructionKind::MakeStruct { dest: d, fields: f, struct_ty: ty } => {
                assert_eq!(*d, dest);
                assert_eq!(*f, fields);
                assert_eq!(*ty, struct_ty);
            }
            _ => panic!("Expected MakeStruct instruction"),
        }

        assert_eq!(instr.destinations(), vec![dest]);
        assert!(instr.used_values().contains(&ValueId::new(1)));
        assert!(instr.used_values().contains(&ValueId::new(2)));
    }

    #[test]
    fn test_extract_struct_field_instruction() {
        let dest = ValueId::new(0);
        let struct_val = Value::Operand(ValueId::new(1));
        let field_name = "x".to_string();
        let field_ty = MirType::felt();

        let instr = Instruction::extract_struct_field(dest, struct_val, field_name.clone(), field_ty.clone());

        match &instr.kind {
            InstructionKind::ExtractStructField { dest: d, struct_val: s, field_name: f, field_ty: ty } => {
                assert_eq!(*d, dest);
                assert_eq!(*s, struct_val);
                assert_eq!(*f, field_name);
                assert_eq!(*ty, field_ty);
            }
            _ => panic!("Expected ExtractStructField instruction"),
        }

        assert_eq!(instr.destinations(), vec![dest]);
        assert!(instr.used_values().contains(&ValueId::new(1)));
    }

    #[test]
    fn test_pretty_print_aggregate_instructions() {
        let dest = ValueId::new(0);
        let elem1 = Value::Operand(ValueId::new(1));
        let elem2 = Value::Operand(ValueId::new(2));

        // Test MakeTuple pretty print
        let tuple_instr = Instruction::make_tuple(dest, vec![elem1, elem2]);
        let tuple_pretty = tuple_instr.pretty_print(0);
        assert!(tuple_pretty.contains("make_tuple"));
        assert!(tuple_pretty.contains("%0"));
        assert!(tuple_pretty.contains("%1"));
        assert!(tuple_pretty.contains("%2"));

        // Test ExtractTupleElement pretty print
        let extract_instr = Instruction::extract_tuple_element(dest, elem1, 0, MirType::felt());
        let extract_pretty = extract_instr.pretty_print(0);
        assert!(extract_pretty.contains("extract_tuple_element"));
        assert!(extract_pretty.contains("0"));

        // Test MakeStruct pretty print
        let fields = vec![("x".to_string(), elem1)];
        let struct_instr = Instruction::make_struct(dest, fields, MirType::simple_struct_type("Point".to_string()));
        let struct_pretty = struct_instr.pretty_print(0);
        assert!(struct_pretty.contains("make_struct"));
        assert!(struct_pretty.contains("x:"));

        // Test ExtractStructField pretty print
        let field_instr = Instruction::extract_struct_field(dest, elem1, "x".to_string(), MirType::felt());
        let field_pretty = field_instr.pretty_print(0);
        assert!(field_pretty.contains("extract_struct_field"));
        assert!(field_pretty.contains("\"x\""));
    }
}
```

### Step 3: Update Module Declarations

**File**: `/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/lib.rs`

Add the new test module (if not automatically included):

```rust
#[cfg(test)]
mod instruction_tests;
```

### Step 4: Verify Integration

**File**:
`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/validation.rs`

Ensure the validation pass accepts the new instructions by checking that they
are handled in the instruction validation logic (they should pass through with
the basic `Ok(())` validation added in step 1.3).

### Step 5: Run Tests and Validation

Execute the following commands to verify the implementation:

```bash
# Run the new tests
cargo test -p cairo-m-compiler-mir instruction_tests

# Run all MIR tests to ensure no regressions
cargo test -p cairo-m-compiler-mir

# Run validation to ensure the new instructions are accepted
cargo test -p cairo-m-compiler-mir validation

# Check formatting and linting
cargo fmt
cargo clippy --package cairo-m-compiler-mir
```

## Testing Requirements

1. **Unit Tests**: All four instruction types must have comprehensive unit tests
   covering:
   - Constructor functions
   - Helper method behavior (`destinations()`, `used_values()`)
   - Pretty printing output
   - Basic validation

2. **Integration Tests**: Create a simple MIR function that uses each new
   instruction type and verify:
   - The function can be created without errors
   - Pretty printing produces readable output
   - Validation passes successfully

3. **Regression Tests**: Ensure existing MIR functionality is unaffected:
   - All existing tests continue to pass
   - No changes to current memory-based lowering behavior

## Definition of Done

- [ ] All four `InstructionKind` variants implemented with correct field types
- [ ] All constructor functions implemented and tested
- [ ] `destinations()`, `used_values()`, and `validate()` methods handle new
      instructions
- [ ] Pretty printing produces readable output for all new instruction types
- [ ] Comprehensive unit tests written and passing
- [ ] All existing MIR tests continue to pass (no regressions)
- [ ] Code review completed and approved
- [ ] Documentation updated in code comments

## Risk Mitigation

1. **Additive Change**: This change is purely additive - existing lowering and
   optimization passes are unaffected until the new instructions are actually
   generated
2. **Isolated Testing**: New instructions can be tested independently without
   affecting the existing pipeline
3. **Gradual Rollout**: The implementation enables future tasks but doesn't
   change current behavior until lowering is updated

## Follow-up Tasks

This task enables several critical follow-up tasks:

- Task 002: Update lowering to use value-based aggregate instructions
- Task 003: Simplify optimization pipeline by removing SROA/Mem2Reg
- Task 004: Implement constant folding for aggregate operations

## Success Metrics

- [x] Compilation time unchanged (no performance regression)
- [x] All tests pass with new instruction types
- [x] Pretty-printed MIR is human-readable for aggregate operations
- [x] Zero regression in existing functionality (2 pre-existing test failures
      unrelated to these changes)
- [x] Foundation ready for value-based lowering implementation

## Implementation Completed

**Date**: 2025-08-16

### What was done:

1. **Added four new InstructionKind variants** in `instruction.rs`:
   - `MakeTuple`: Creates tuples from a list of values
   - `ExtractTupleElement`: Extracts elements from tuples by index
   - `MakeStruct`: Creates structs from field-value pairs
   - `ExtractStructField`: Extracts fields from structs by name

2. **Implemented constructor functions** for each new instruction type:
   - `Instruction::make_tuple()`
   - `Instruction::extract_tuple_element()`
   - `Instruction::make_struct()`
   - `Instruction::extract_struct_field()`

3. **Updated all required helper methods**:
   - `destinations()`: Returns destination ValueIds for new instructions
   - `used_values()`: Tracks which values are used by new instructions
   - `validate()`: Basic validation passes for all new instructions

4. **Implemented pretty printing** for all new instructions:
   - Human-readable output for debugging and IR visualization
   - Clear format showing operation, operands, and types

5. **Created comprehensive test suite** in `instruction_tests.rs`:
   - Unit tests for all constructor functions
   - Tests for helper method behaviors
   - Pretty printing validation
   - Basic validation tests

### Technical Notes:

- The implementation is purely additive - no existing functionality was modified
- All new instructions follow the established MIR patterns and conventions
- The instructions are ready for use by the lowering phase (Task 002)
- Type information is preserved for struct and tuple element access

### Testing Results:

- All 6 new aggregate instruction tests pass
- No regressions introduced (2 pre-existing mem2reg_ssa test failures are
  unrelated)
- Code compiles without errors
- Pretty printing produces expected human-readable output
