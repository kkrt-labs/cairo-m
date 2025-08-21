# Fix Constant Folding Semantics for Felt and U32

**Priority**: HIGH **Component**: MIR Passes **Impact**: Correctness (Silent
Miscompilation)

## Problem

The constant folding implementation in
`crates/compiler/mir/src/passes/constant_folding.rs` has critical semantic bugs:

### 1. Felt Arithmetic is Wrong

> Note: double check this; as it might have been fixed already.

- Lines 42-64: `felt` arithmetic uses regular integer operations instead of
  modular field arithmetic
- **Issue**: `felt` is a prime-field element (mod p = 2^31 - 1). All arithmetic
  must be `mod p`
- **Division bug**: Uses truncating integer division `/` instead of modular
  inverse multiplication
- **Impact**: Programs will silently miscompile and produce wrong results

### 2. U32 Arithmetic has Lossy Casts

- Lines 92-104: U32 operations cast to `u32`, operate, then cast back to `i32`
- **Issue**: Values ≥ 2^31 become negative `i32` values
- **Impact**: U32 values > i32::MAX will fold incorrectly

```rust
// Current broken code:
Some(Literal::Integer((a.wrapping_add(b)) as i32))  // ❌ Lossy cast
```

## Solution

### Phase 1: Fix Current Implementation

1. **Felt arithmetic**: Use M31 field operations consistently
2. **U32 arithmetic**: Keep values as u32, don't cast to i32
3. **Add tests** for edge cases (large values, division, overflow)

### Phase 2: Type-Aware Literals (Recommended)

Replace `Literal::Integer(i32)` with type-aware variants:

```rust
pub enum Literal {
    Felt(M31),      // Use M31 directly for field arithmetic
    U32(u32),       // Keep as u32, no lossy casts
    Boolean(bool),
    Unit,
}
```

## Files to Modify

- `crates/compiler/mir/src/value.rs` - Update Literal enum
- `crates/compiler/mir/src/passes/constant_folding.rs` - Fix arithmetic
- Add tests in `crates/compiler/mir/src/passes/constant_folding_tests.rs`

## Test Cases Needed

1. **Felt**: Large values that would overflow i32
2. **Felt**: Division with modular inverse
3. **U32**: Values > i32::MAX (e.g., `u32::MAX + 1 = 0`)
4. **U32**: All operations near overflow boundaries

## Dependencies

- Ensure M31 field operations are available and tested
- May need to update other passes that consume Literal values

## Acceptance Criteria

- [ ] Felt arithmetic uses proper modular field operations
- [ ] U32 arithmetic preserves full 32-bit range without sign issues
- [ ] All existing tests continue to pass
- [ ] New edge case tests pass
- [ ] No silent miscompilation of field or u32 arithmetic
