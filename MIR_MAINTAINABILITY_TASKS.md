# MIR Maintainability Improvements - Task List

## Priority 1: Eliminate Code Duplication (High Impact)

### Task 1.1: Refactor Value Visitor Pattern

**Impact**: Reduces ~30% of repetitive code across instruction and terminator
handling **Files**: `crates/compiler/mir/src/instruction.rs`,
`crates/compiler/mir/src/terminator.rs`

- [ ] Create a macro or helper trait for visiting `Value` operands
- [ ] Apply to `Instruction::used_values()` method
- [ ] Apply to `Instruction::replace_value_uses()` method
- [ ] Apply to `Terminator::used_values()` method
- [ ] Apply to `Terminator::replace_value_uses()` method

**Implementation sketch**:

```rust
macro_rules! visit_operands {
    ($value:expr, $visitor:expr) => {
        if let Value::Operand(id) = $value {
            $visitor(id);
        }
    };
}
```

### Task 1.2: Unify Array/Aggregate Type Checking

**Impact**: Single source of truth for type system decisions **Files**:
`crates/compiler/mir/src/mir_types.rs`,
`crates/compiler/mir/src/lowering/array_guards.rs`

- [ ] Remove `array_guards.rs` module entirely
- [ ] Update all `should_use_memory_lowering()` calls to use
      `MirType::requires_memory_path()`
- [ ] Update all `supports_value_aggregates()` calls to use
      `MirType::uses_value_aggregates()`
- [ ] Test that behavior remains identical

## Priority 2: API Consistency & Safety

### Task 2.1: Fix InstrBuilder::call Signature Bug

**Impact**: Prevents incorrect call signatures that could break type checking
**File**: `crates/compiler/mir/src/builder/instr_builder.rs`

- [ ] Deprecate `InstrBuilder::call()` method with warning
- [ ] Update all call sites to use `emit_call_with_destinations()` instead
- [ ] Remove deprecated method after migration
- [ ] Add test to verify correct parameter type inference

### Task 2.2: Clean Up Dead/Unused Code

**Impact**: Reduces maintenance burden and confusion **Files**: Various

- [ ] Remove unused `AccessPath` type from `instruction.rs`
- [ ] Remove unused `FieldPath` type from `instruction.rs`
- [ ] Either implement or remove `ModuleDebugInfo` stub
- [ ] Either implement or rename `PipelineConfig::from_environment()` to
      `default()`

## Priority 3: Error Handling Improvements

### Task 3.1: Convert Panics to Result Types

**Impact**: Makes the compiler more robust and user-friendly **Files**:
`crates/compiler/mir/src/lowering/stmt.rs`,
`crates/compiler/mir/src/lowering/expr.rs`

- [ ] Change array assignment panic to return `Err(String)`
- [ ] Change array index access panic to return `Err(String)`
- [ ] Update callers to propagate errors properly
- [ ] Add user-friendly error messages with source locations

### Task 3.2: Propagate Function Lowering Errors

**Impact**: Prevents silent failures and half-baked modules **File**:
`crates/compiler/mir/src/lowering/function.rs`

- [ ] Change `generate_mir()` to collect all function errors
- [ ] Return `Err(Vec<Diagnostic>)` if any function fails

## Priority 5: Documentation & Testing

### Task 5.1: Document Type System Invariants

**Impact**: Prevents future confusion about array semantics **File**:
`crates/compiler/mir/src/mir_types.rs`

- [ ] Document aggregate value semantics
- [ ] Add examples of correct usage patterns
- [ ] Document data layout assumptions (U32 size=2, etc.)

### Task 5.2: Add Debug IR Dumping

**Impact**: Dramatically improves debugging of optimization issues **File**:
`crates/compiler/mir/src/pipeline.rs`. Add to README.md instructions for how to
debug eventual issues with passes.

- [ ] When `PipelineConfig::debug == true`, dump MIR between passes
- [ ] Include pass name in dump filename
- [ ] Add option to dump only on changes
- [ ] Add option to dump specific functions only

## Estimated Impact Summary

1. **Code Reduction**: ~500-800 lines removed through deduplication
2. **Bug Prevention**: 5 panic sites converted to proper errors
3. **Performance**: 20-30% more optimizations from fixed-point iteration
4. **Developer Experience**: Clearer APIs, better debugging, single source of
   truth

## Dependencies Between Tasks

- Task 1.1 should be done before any major refactoring of
  instructions/terminators
- Task 1.2 blocks any new array/aggregate features
- Task 3.1 and 3.2 should be done together for consistent error handling
