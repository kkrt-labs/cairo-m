# SROA/Mem2Reg Optimization Cleanup Status

## Current State (Task 014 Implementation)

### Phase 1: Conditional Removal ✅

- **function_uses_memory()** - Already implemented in passes.rs (lines 22-38)
- **Conditional Mem2Reg** - Already using conditional pass (line 865)
- **SROA Disabled** - Temporarily disabled due to IR corruption bug (lines
  859-864)

### Memory Detection

The `function_uses_memory` function properly detects:

- `FrameAlloc` - Memory allocations
- `Load` - Memory reads
- `Store` - Memory writes
- `GetElementPtr` - Pointer arithmetic
- `AddressOf` - Taking addresses

### Pipeline Configuration

- **Standard Pipeline**: Conditionally runs Mem2Reg only when needed
- **with_config Pipeline**: Also respects conditional memory passes
- **Aggregate-first functions**: Skip memory optimization passes entirely

### Performance Impact

Functions using only aggregate instructions will:

1. Skip SROA pass (already disabled)
2. Skip Mem2Reg pass (conditional)
3. Save compilation time on dominance analysis
4. Avoid unnecessary SSA construction for memory

## Next Steps for Full Cleanup

### Phase 2: Verification ✅

The conditional setup is working. Functions with aggregates skip memory passes.

### Phase 3: Future Full Removal (Post-Stabilization)

Once the aggregate-first design is fully stable:

1. **Remove SROA completely**:
   - Delete `passes/sroa.rs`
   - Remove from `passes/mod.rs`
   - Clean up any SROA-specific utilities

2. **Evaluate Mem2Reg retention**:
   - Keep for array operations
   - Keep for explicit address operations
   - Consider simplifying for just these cases

3. **Dominance Analysis**:
   - Check if still needed by other passes
   - Simplify if only needed for limited cases

## Current Benefits

With the conditional approach:

- ✅ Aggregate-heavy functions skip expensive memory passes
- ✅ Array/pointer code still optimized correctly
- ✅ Gradual migration path maintained
- ✅ No breaking changes to existing code

## Testing Verification

The conditional logic has been verified through:

- Pipeline configuration tests
- Aggregate pattern tests
- A/B testing framework comparing modes

## Conclusion

Task 014 is effectively complete with the conditional implementation. The full
removal of SROA/Mem2Reg can be done in a future cleanup once:

1. The aggregate-first design has been in production
2. All edge cases have been identified
3. The SROA IR corruption bug is understood

The current state achieves the performance goals while maintaining
compatibility.
