# MIR Crate Refactoring Tasks

This directory contains detailed task descriptions for completing the transition
to a value-based, aggregate-first MIR design and cleaning up legacy code.

## Task Overview

The MIR crate has partially implemented a value-based aggregate system but
remains in a hybrid state. These tasks complete the transition and remove
technical debt.

## Task Priorities

### 🔴 CRITICAL (Blocks Value-Based Design)

These tasks must be completed to achieve the aggregate-first design goals:

1. **[001-critical-variable-ssa-pass.md](001-critical-variable-ssa-pass.md)**
   - Implement Variable-SSA pass for proper SSA form with Phi nodes
   - Enables variable rebinding without memory operations

2. **[002-critical-assignment-ssa-rebinding.md](002-critical-assignment-ssa-rebinding.md)**
   - Convert assignments from memory stores to SSA rebinding
   - Use InsertField/InsertTuple for aggregate updates

3. **[003-critical-function-call-tuple-synthesis.md](003-critical-function-call-tuple-synthesis.md)**
   - Synthesize tuples for multi-return functions using MakeTuple
   - Eliminate memory spilling for function returns

### 🟡 HIGH PRIORITY (Technical Debt)

Important cleanup tasks that improve code quality:

4. **[004-high-sroa-cleanup.md](004-high-sroa-cleanup.md)**
   - Remove or fix SROA pass (currently disabled due to IR corruption)
   - Recommendation: Remove entirely as incompatible with new design

5. **[005-high-deprecated-api-removal.md](005-high-deprecated-api-removal.md)**
   - Complete migration from deprecated memory-based APIs
   - Remove load_field, store_field, load_tuple_element, store_tuple_element

### 🟢 MEDIUM PRIORITY (Optimization)

Performance and maintainability improvements:

6. **[006-medium-optimization-pipeline-refactor.md](006-medium-optimization-pipeline-refactor.md)**
   - Refactor pipeline for aggregate-first design
   - Add aggregate-specific optimization passes

7. **[007-medium-dead-store-elimination-fix.md](007-medium-dead-store-elimination-fix.md)**
   - Fix soundness issue with GEP-derived pointers
   - Implement basic alias analysis

### ⚪ LOW PRIORITY (Cleanup)

Minor improvements and documentation:

8. **[008-low-cleanup-todos.md](008-low-cleanup-todos.md)**
   - Address TODO comments throughout codebase
   - Update documentation and remove obsolete code

## Current State Summary

### ✅ Successfully Implemented

- First-class aggregate instructions (MakeTuple, MakeStruct, etc.)
- Value-based lowering for literals and field access
- Conditional optimization pass execution
- Validation and pretty-printing for aggregates

### ❌ Not Implemented (Critical Gaps)

- Variable-SSA pass for mutable state management
- SSA rebinding for assignments
- Tuple synthesis for function returns
- Full removal of deprecated APIs

### ⚠️ Hybrid State Issues

- R-values use value-based operations ✅
- L-values still use memory operations ❌
- Optimization pipeline partially adapted
- Some passes disabled or broken

## Implementation Order

### Phase 1: Core SSA Infrastructure

1. Implement Variable-SSA pass (Task 001)
2. Convert assignments to SSA (Task 002)
3. Fix function call returns (Task 003)

### Phase 2: Cleanup

4. Remove deprecated APIs (Task 005)
5. Remove/fix SROA (Task 004)

### Phase 3: Optimization

6. Refactor optimization pipeline (Task 006)
7. Fix dead store elimination (Task 007)

### Phase 4: Polish

8. Clean up TODOs and documentation (Task 008)

## Testing Strategy

Each task includes specific test requirements. Overall testing approach:

1. **Unit Tests**: Test individual components in isolation
2. **Integration Tests**: Verify end-to-end compilation
3. **Snapshot Tests**: Ensure IR generation is correct
4. **Performance Tests**: Measure compilation and runtime impact
5. **Regression Tests**: Ensure no existing functionality breaks

## Success Metrics

The refactoring is complete when:

1. **No memory operations** for tuple/struct state management
2. **All tests pass** with value-based implementation
3. **Performance improved** by >10% for aggregate-heavy code
4. **Zero deprecated API usage** in codebase
5. **Clean pipeline** with aggregate-aware optimizations

## Quick Reference

### Files to Modify Most

- `crates/compiler/mir/src/lowering/stmt.rs` - Assignment and return handling
- `crates/compiler/mir/src/lowering/expr.rs` - Function call expressions
- `crates/compiler/mir/src/passes.rs` - Optimization pipeline
- `crates/compiler/mir/src/lowering/builder.rs` - Deprecated methods

### Key Patterns to Change

#### Before (Memory-Based)

```rust
let addr = lower_lvalue_expression(expr);
store(addr, value);
load_tuple_element(addr, index);
```

#### After (Value-Based)

```rust
let value = lower_expression(expr);
rebind_variable(var, value);
extract_tuple_element(value, index);
```

## Notes

- Tasks are designed to be somewhat independent where possible
- Critical tasks (001-003) are interdependent and should be done together
- Each task includes verification checklists and success criteria
- Conservative time estimates included for planning purposes
