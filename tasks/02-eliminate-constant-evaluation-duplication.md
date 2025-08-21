# Eliminate Constant Evaluation Logic Duplication

**Priority**: HIGH  
**Component**: MIR Passes  
**Impact**: Maintainability, Correctness

## Problem

Constant evaluation logic is duplicated across multiple passes, leading to:

- **Semantic drift**: Different passes handle edge cases differently
- **Maintenance burden**: Same bugs need fixing in multiple places
- **Inconsistency**: Felt semantics already diverged between passes

### Affected Files

- `passes/constant_folding.rs` - Main constant folding
- `passes/simplify_branches.rs` - `evaluate_comparison` method
- `passes/arithmetic_simplify.rs` - Boolean algebra & identities
- `passes/fuse_cmp.rs` - Zero optimizations
- Future passes will need similar logic

### Current Duplication Examples

1. **Comparison evaluation** in both ConstantFolding and SimplifyBranches
2. **Boolean algebra** reimplemented in ArithmeticSimplify
3. **Zero checks** scattered across multiple passes
4. **Type coercion** handled differently in each pass

## Solution

### Create Centralized Const Eval Module

**New file**: `crates/compiler/mir/src/passes/const_eval.rs`

```rust
pub struct ConstEvaluator {
    // Caches, configuration, etc.
}

impl ConstEvaluator {
    /// Evaluate binary operation on literals
    pub fn eval_binary_op(
        &self,
        op: BinaryOp,
        left: Literal,
        right: Literal
    ) -> Option<Literal>;

    /// Evaluate unary operation on literal
    pub fn eval_unary_op(
        &self,
        op: UnaryOp,
        operand: Literal
    ) -> Option<Literal>;

    /// Evaluate comparison, returns boolean literal
    pub fn eval_comparison(
        &self,
        op: BinaryOp,
        left: Literal,
        right: Literal
    ) -> Option<bool>;

    /// Check if value is zero for any type
    pub fn is_zero(&self, value: &Value) -> bool;

    /// Convert literal to boolean (for branch conditions)
    pub fn as_bool(&self, literal: Literal) -> Option<bool>;
}
```

### Domain-Correct Implementation

1. **Felt arithmetic**: Use M31 field operations with proper modulus
2. **U32 arithmetic**: Keep values in u32 domain
3. **Boolean logic**: Standard boolean algebra
4. **Comparisons**: Type-appropriate comparison semantics

### Migration Plan

**Phase 1**: Create const*eval module with correct semantics **Phase 2**:
Replace ConstantFolding::try_fold*\* methods  
**Phase 3**: Replace SimplifyBranches::evaluate_comparison **Phase 4**: Replace
boolean algebra in ArithmeticSimplify **Phase 5**: Replace zero checks in
FuseCmp

## Files to Modify

- **New**: `crates/compiler/mir/src/passes/const_eval.rs`
- **Update**: `crates/compiler/mir/src/passes/constant_folding.rs`
- **Update**: `crates/compiler/mir/src/passes/simplify_branches.rs`
- **Update**: `crates/compiler/mir/src/passes/arithmetic_simplify.rs`
- **Update**: `crates/compiler/mir/src/passes/fuse_cmp.rs`
- **Tests**: Add comprehensive const_eval tests

## Benefits

1. **Single source of truth** for constant evaluation
2. **Consistent semantics** across all passes
3. **Easier testing** - test once, reuse everywhere
4. **Maintenance** - fix bugs in one place
5. **Performance** - shared optimizations and caching

## Test Strategy

- **Unit tests** for const_eval module covering all operations
- **Integration tests** ensuring all passes produce consistent results
- **Edge case tests** for overflow, underflow, division by zero
- **Cross-validation** between old and new implementations during migration

## Dependencies

- Must complete after fixing constant folding semantics (Task #1)
- Should coordinate with type-aware literal implementation

## Acceptance Criteria

- [ ] Central const_eval module implemented with correct semantics
- [ ] All passes migrated to use const_eval
- [ ] No duplication of constant evaluation logic
- [ ] All existing tests pass
- [ ] New comprehensive const_eval test suite
- [ ] Performance neutral or improved
