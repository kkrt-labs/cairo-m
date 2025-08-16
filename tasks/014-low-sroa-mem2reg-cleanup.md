# Task 014: SROA and Mem2Reg Cleanup (LOW PRIORITY)

**Priority:** LOW  
**Dependencies:** All aggregate tasks (final cleanup)  
**Issue Reference:** MIR_REPORT.md Issue 17

## Why

Once the aggregate infrastructure is complete and all lowering produces
value-based MIR, the SROA (Scalar Replacement of Aggregates) and Mem2Reg SSA
passes become obsolete. These passes were specifically designed to reverse the
"all aggregates in memory" lowering strategy by converting memory operations
back into SSA values.

With the new aggregate-first design:

- Tuples and structs are created directly as SSA values using
  `MakeTuple`/`MakeStruct`
- Field and element access uses `ExtractField`/`ExtractTuple` operations
- No intermediate memory allocation occurs for simple aggregates
- The Var-SSA pass handles variable rebinding without memory

Removing these passes eliminates:

- Complex dominance frontier analysis overhead
- SSA renaming walks through the entire function
- Memory-to-SSA conversion logic that's no longer needed
- Significant compile-time overhead for functions that never use memory

This cleanup represents the primary payoff of the MIR refactoring effort - a
simpler, more efficient compilation pipeline.

## What

Remove obsolete optimization passes and related infrastructure:

### Core Pass Removal

- Delete `passes/sroa.rs` - Scalar Replacement of Aggregates pass
- Delete `passes/mem2reg_ssa.rs` - Memory-to-Register SSA conversion pass
- Remove pipeline registration of these passes

### Dominance Analysis Cleanup

- Evaluate `analysis/dominance.rs` for continued use by Var-SSA
- If Var-SSA uses different dominance logic, remove the old implementation
- Keep only what's needed for remaining memory-based operations (arrays,
  explicit addresses)

### Build System Updates

- Remove module declarations from `passes/mod.rs`
- Remove module declarations from `analysis/mod.rs`
- Update any build dependencies that were specific to these passes

### Documentation and Comments

- Remove references to SROA/Mem2Reg in code comments
- Update pipeline documentation
- Clean up any design documents mentioning these passes

## How

### Phase 1: Conditional Removal Setup

1. **Add Feature Flag Detection**

   ```rust
   // In passes.rs
   fn function_uses_memory(function: &MirFunction) -> bool {
       function.blocks.values().any(|block| {
           block.instructions.iter().any(|instr| {
               matches!(instr.kind,
                   InstructionKind::FrameAlloc { .. } |
                   InstructionKind::Load { .. } |
                   InstructionKind::Store { .. } |
                   InstructionKind::GetElementPtr { .. }
               )
           })
       })
   }
   ```

2. **Modify Pipeline Registration**
   ```rust
   // In PassManager::standard_pipeline()
   // Comment out or conditionally include:
   if function_uses_memory(function) {
       // Only run for functions that still use memory operations
       pipeline.add_pass(Box::new(sroa::SroaPass));
       pipeline.add_pass(Box::new(mem2reg_ssa::Mem2RegSsaPass));
   }
   ```

### Phase 2: Verification Period

1. **Run Test Suite**
   - Execute all compiler tests with conditional removal
   - Verify functions using new aggregate lowering skip SROA/Mem2Reg
   - Ensure memory-based functions (arrays, addresses) still work correctly

2. **Performance Validation**
   - Measure compilation time improvements for aggregate-heavy code
   - Verify generated code quality remains equivalent
   - Test with both small and large codebases

### Phase 3: Full Removal

1. **Delete Pass Files**

   ```bash
   rm crates/compiler/mir/src/passes/sroa.rs
   rm crates/compiler/mir/src/passes/mem2reg_ssa.rs
   ```

2. **Update Module Declarations**

   ```rust
   // Remove from passes/mod.rs:
   // pub mod sroa;
   // pub mod mem2reg_ssa;
   ```

3. **Clean Up Dominance Analysis**
   - Audit `analysis/dominance.rs` usage
   - If only used by removed passes, delete the file
   - If used by Var-SSA, keep minimal implementation

   ```rust
   // Remove from analysis/mod.rs if unused:
   // pub mod dominance;
   ```

4. **Pipeline Cleanup**
   ```rust
   // Remove from PassManager::standard_pipeline():
   // pipeline.add_pass(Box::new(sroa::SroaPass));
   // pipeline.add_pass(Box::new(mem2reg_ssa::Mem2RegSsaPass));
   ```

### Phase 4: Documentation Updates

1. **Update Code Comments**
   - Remove references to SROA/Mem2Reg in lowering code
   - Update pipeline documentation
   - Clean up optimization pass documentation

2. **Update Design Documents**
   - Revise MIR design documentation
   - Update optimization pipeline descriptions
   - Document the new aggregate-first approach

3. **Performance Documentation**
   - Document compilation time improvements
   - Note simplified optimization pipeline
   - Update debugging guides for new MIR structure

### Phase 5: Final Verification

1. **Comprehensive Testing**
   - Run full test suite including integration tests
   - Test compilation of large real-world programs
   - Verify error handling and diagnostics still work

2. **Performance Benchmarks**
   - Measure and document compilation speed improvements
   - Verify memory usage reduction during compilation
   - Test with various code patterns (heavy aggregates, minimal aggregates)

## Implementation Notes

### Timing Considerations

- This should be the **final** task in the aggregate refactoring
- Only proceed after all other aggregate tasks are complete and stable
- Allow for a settling period with conditional removal before full deletion

### Compatibility

- Keep support for memory-based operations for arrays and explicit addresses
- Ensure the simplified pipeline can still handle mixed code patterns
- Maintain backward compatibility for existing test cases

### Error Handling

- Ensure diagnostic quality doesn't degrade
- Verify error messages remain helpful without SROA/Mem2Reg context
- Test edge cases that previously relied on these passes

### Future Considerations

- Consider keeping minimal dominance analysis for future optimizations
- Plan for potential backend requirements that might need memory operations
- Document the decision-making process for future maintainers

## Success Criteria

1. **Functional Success**
   - All existing tests pass without SROA/Mem2Reg passes
   - New aggregate-based code compiles correctly
   - Memory-based operations (arrays, addresses) still work

2. **Performance Success**
   - Measurable reduction in compilation time for aggregate-heavy code
   - No regression in generated code quality
   - Reduced memory usage during compilation

3. **Code Quality Success**
   - Simplified pipeline is easier to understand and maintain
   - Reduced codebase complexity
   - Cleaner separation between memory-based and value-based operations

4. **Documentation Success**
   - Clear documentation of the new optimization pipeline
   - Updated design documents reflect aggregate-first approach
   - Migration guide for contributors familiar with old system
