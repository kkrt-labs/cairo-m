# MIR Critical Bugs - Task List

## Bug 1: Pretty Printer Breaks Round-Tripping (CRITICAL)

**Severity**: High - Breaks MIR text format compatibility **Files**:
`crates/compiler/mir/src/instruction.rs`,
`crates/compiler/mir/src/terminator.rs`

### Tasks:

- [ ] Fix `BinaryOp` formatting in `Instruction::PrettyPrint` to use Display
      trait
- [ ] Fix `BranchCmp` formatting in `Terminator::PrettyPrint` to use Display
      trait
- [ ] Add round-trip test: parse MIR → pretty-print → parse again

**Fix for instruction.rs (line ~800)**:

```rust
// BEFORE: format!("{:?}", op)
// AFTER:
result.push_str(&format!(
    "{} = {} {} {}",  // op uses Display, not Debug
    dest.pretty_print(0),
    left.pretty_print(0),
    op,
    right.pretty_print(0)
));
```

**Fix for terminator.rs (line ~250)**:

```rust
// BEFORE: format!("{:?}", op)
// AFTER:
format!(
    "if {} {} {} then jump {then_target:?} else jump {else_target:?}",
    left.pretty_print(0),
    op,  // Uses Display
    right.pretty_print(0)
)
```

## Bug 2: Call Instruction Type Safety Violation (CRITICAL)

**Severity**: High - Can corrupt SSA form in release builds **File**:
`crates/compiler/mir/src/instruction.rs`

### Tasks:

- [ ] Replace `debug_assert!` with `assert_eq!` in `Instruction::call()`
- [ ] Add clear error message with signature details
- [ ] Add test for mismatched call signatures

**Fix (line ~150)**:

```rust
pub fn call(...) -> Self {
    // BEFORE: debug_assert_eq!(dests.len(), signature.return_types.len());
    // AFTER:
    assert_eq!(
        dests.len(),
        signature.return_types.len(),
        "Call instruction: destination count ({}) must match return types ({})",
        dests.len(),
        signature.return_types.len()
    );
    // ...
}
```

## Bug 3: Silent CFG Corruption on Invalid Block ID (CRITICAL)

**Severity**: High - Hides logic bugs, corrupts control flow **File**:
`crates/compiler/mir/src/builder/cfg_builder.rs`

### Tasks:

- [ ] Replace silent return with `expect()` or panic with message
- [ ] Add block ID validation helper
- [ ] Add debug assertion for valid block IDs

**Fix for set_terminator_internal (line ~180)**:

```rust
fn set_terminator_internal(&mut self, block_id: BasicBlockId, terminator: Terminator) {
    // BEFORE: if invalid { return; }
    // AFTER:
    let old_targets = self.function.basic_blocks
        .get(block_id)
        .expect(&format!("set_terminator_internal: invalid block_id {:?}", block_id))
        .terminator
        .target_blocks();
    // ...
}
```

## Bug 4: Missing Return Type Validation (HIGH)

**Severity**: High - Type system unsoundness **File**:
`crates/compiler/mir/src/function.rs`

### Tasks:

- [ ] Add return value validation in `MirFunction::validate()`
- [ ] Check count matches signature
- [ ] Check types match signature
- [ ] Add test cases for mismatched returns

**Implementation for validate() method**:

```rust
pub fn validate(&self) -> Result<(), String> {
    // ... existing validation ...

    // Add return validation
    for (block_id, block) in self.basic_blocks() {
        if let Terminator::Return { values } = &block.terminator {
            // Check count
            if values.len() != self.return_values.len() {
                return Err(format!(
                    "Block {:?}: return has {} values, expected {}",
                    block_id, values.len(), self.return_values.len()
                ));
            }

            // Check types
            for (i, value) in values.iter().enumerate() {
                if let Value::Operand(id) = value {
                    let actual_type = self.get_value_type(*id)?;
                    let expected_id = self.return_values[i];
                    let expected_type = self.get_value_type(expected_id)?;

                    if actual_type != expected_type {
                        return Err(format!(
                            "Block {:?}: return value {} has type {:?}, expected {:?}",
                            block_id, i, actual_type, expected_type
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}
```

## Bug 5: Swallowed Lowering Errors (HIGH)

**Severity**: High - Produces invalid modules silently **File**:
`crates/compiler/mir/src/lowering/function.rs`

### Tasks:

- [ ] Change `generate_mir()` to collect all errors
- [ ] Return `Err(Vec<Diagnostic>)` on any failure
- [ ] Add flag for continuing on error (dev mode)
- [ ] Ensure no partial modules escape

**Fix for generate_mir**:

```rust
pub fn generate_mir(db: &dyn MirDb, crate_id: Crate) -> Result<MirModule, Vec<Diagnostic>> {
    let mut module = MirModule::new(crate_name);
    let mut errors = Vec::new();

    for function in functions {
        match lower_function(db, function) {
            Ok(mir_func) => module.add_function(mir_func),
            Err(e) => {
                errors.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: format!("Failed to lower function {}: {}", function.name, e),
                    // ... location info
                });
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(module)
}
```

## Bug 6: InstrBuilder Creates Invalid Call Signatures (MEDIUM-HIGH)

**Severity**: Medium-High - Incorrect type information **File**:
`crates/compiler/mir/src/builder/instr_builder.rs`

### Tasks:

- [ ] Deprecate `InstrBuilder::call()` with `#[deprecated]`
- [ ] Redirect all usage to `emit_call_with_destinations()`
- [ ] Remove method in next version
- [ ] Document why it was removed

**Immediate fix**:

```rust
#[deprecated(since = "0.2.0", note = "Use emit_call_with_destinations instead - this method creates incorrect signatures")]
pub fn call(&mut self, callee: FunctionId, args: Vec<Value>, return_types: Vec<MirType>) -> Vec<ValueId> {
    // Either compute correct param_types or just panic
    panic!("InstrBuilder::call is deprecated due to signature bugs. Use emit_call_with_destinations instead");
}
```

## Verification Checklist

After fixing each bug:

1. [ ] Run existing test suite
2. [ ] Add specific regression test for the bug
3. [ ] Run with debug assertions enabled
4. [ ] Test with a complex real-world program
5. [ ] Verify fix doesn't break other functionality

## Priority Order

1. **Bug 2** (Call instruction) - Data corruption risk
2. **Bug 4** (Return validation) - Type system hole
3. **Bug 1** (Pretty printer) - Breaks tooling
4. **Bug 3** (CFG corruption) - Hidden failures
5. **Bug 5** (Swallowed errors) - Silent failures
6. **Bug 6** (InstrBuilder) - API correctness

## Testing Strategy

For each bug fix:

- Unit test the specific failure case
- Integration test with full compilation pipeline
- Fuzz test with random MIR generation (if applicable)
- Performance test to ensure no regression
