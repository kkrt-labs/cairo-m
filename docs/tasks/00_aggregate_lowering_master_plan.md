# MIR Aggregate Lowering Master Plan for CASM Backend

## Executive Summary

This document outlines the comprehensive plan to fix aggregate value handling in
the Cairo-M MIR compiler for CASM code generation. The goal is to ensure all
aggregate values (structs, tuples) are properly lowered to memory operations or
scalarized before reaching the CASM backend.

## Problem Statement

The CASM backend cannot handle value-based aggregate operations (`MakeStruct`,
`MakeTuple`, `ExtractField`, etc.). Currently, these operations leak through to
codegen causing failures like:

```
CodeGenerationFailed("Invalid MIR: Aggregate value operations should be lowered before code generation")
```

## Root Causes

1. **Pipeline Ordering**: `LowerAggregatesPass` runs AFTER `Mem2RegSsaPass`,
   preventing memory optimizations
2. **GEP Tracking Broken**: `mem2reg` cannot promote struct/tuple fields because
   GEP offset tracking is unimplemented
3. **Inefficient Lowering**: Insert operations always copy entire aggregates
   instead of updating in-place
4. **Missing Optimizations**: No folding for `Extract(Insert(...))` patterns
5. **Dead Code**: Non-functional `VarSsaPass` creates confusion

## Solution Overview

### Core Strategy

- Keep value-based aggregates in high-level MIR for optimization
- Lower aggregates to memory BEFORE mem2reg optimization
- Scalarize non-escaping fields back to registers
- Validate aggregate-free MIR before CASM codegen

### Task Breakdown

| Task                          | Priority | Impact                    | Complexity | Document                                                             |
| ----------------------------- | -------- | ------------------------- | ---------- | -------------------------------------------------------------------- |
| 1. Fix mem2reg GEP tracking   | HIGH     | Enables field promotion   | Medium     | [01_fix_mem2reg_gep_tracking.md](01_fix_mem2reg_gep_tracking.md)     |
| 2. Optimize Insert operations | HIGH     | 83% instruction reduction | Low        | [02_optimize_insert_operations.md](02_optimize_insert_operations.md) |
| 3. Add Insert+Extract folding | MEDIUM   | Eliminates redundant ops  | Low        | [03_add_insert_extract_folding.md](03_add_insert_extract_folding.md) |
| 4. Fix pipeline ordering      | CRITICAL | Fixes root cause          | Medium     | [04_fix_pipeline_ordering.md](04_fix_pipeline_ordering.md)           |
| 5. Remove VarSsaPass          | LOW      | Code cleanup              | Low        | [05_varssa_decision.md](05_varssa_decision.md)                       |
| 6. Add codegen validation     | HIGH     | Early error detection     | Low        | [06_codegen_validation.md](06_codegen_validation.md)                 |

## Implementation Order

### Phase 1: Critical Fixes (Week 1)

1. **Fix pipeline ordering** (Task 4) - Unblocks everything else
2. **Add codegen validation** (Task 6) - Provides immediate feedback
3. **Fix mem2reg GEP tracking** (Task 1) - Enables field promotion

### Phase 2: Optimizations (Week 2)

4. **Optimize Insert operations** (Task 2) - Major performance win
5. **Add Insert+Extract folding** (Task 3) - Eliminates redundancy

### Phase 3: Cleanup (Week 3)

6. **Remove VarSsaPass** (Task 5) - Simplify codebase

## Expected Outcomes

### Performance Improvements

- **83% reduction** in instructions for aggregate updates
- **Elimination** of store→load pairs for non-escaping fields
- **Scalarization** of aggregate fields into registers

### Code Quality

- Clean separation between value-based and memory-based aggregate handling
- Clear pipeline architecture with backend-specific configurations
- Comprehensive validation preventing aggregate leaks to codegen

### Test Success

All failing tests should pass:

- `m_03_types_m_02_structs::nested_structs`
- `m_03_types_m_02_structs::struct_as_function_parameter`
- `m_03_types_m_02_structs::struct_field_access`
- `m_03_types_m_02_structs::struct_field_access_2`

## Success Criteria

1. **No aggregate instructions** in MIR reaching CASM codegen
2. **No spurious store→load pairs** for non-escaping fields
3. **All struct/tuple tests pass** without codegen errors
4. **Performance metrics** show reduced instruction count
5. **Clean validation** catches aggregate leaks early

## Pipeline Architecture

### Before (Problematic)

```
PreOpt → ConstFold → Mem2Reg → SSADestruct → LowerAggregates → Codegen
                        ↑                           ↓
                    Can't see                 Creates memory
                  memory from aggs            too late to optimize
```

### After (Fixed)

```
PreOpt → ConstFold → LowerAggregates → Mem2Reg → SSADestruct → Validation → Codegen
                            ↓              ↑                          ↓
                     Creates memory    Optimizes it           Ensures no aggregates
```

## Risk Mitigation

| Risk                   | Mitigation                               |
| ---------------------- | ---------------------------------------- |
| Breaking existing code | Backward-compatible pipeline modes       |
| Performance regression | Comprehensive benchmarking before/after  |
| Incomplete lowering    | Pre-codegen validation catches issues    |
| Type safety issues     | Maintain strict type checking throughout |

## Testing Strategy

1. **Unit tests** for each optimization pass
2. **Integration tests** for full pipeline
3. **Regression tests** for previously failing cases
4. **Performance benchmarks** for aggregate-heavy code
5. **Validation tests** ensuring aggregate-free output

## Monitoring & Metrics

Track these metrics to measure success:

- Instruction count reduction percentage
- Store/load elimination count
- Test pass rate
- Compilation time impact
- Memory usage change

## Next Steps

1. Review and approve this master plan
2. Create feature branch for implementation
3. Implement Phase 1 (critical fixes)
4. Run benchmarks and tests
5. Continue with Phase 2 and 3
6. Final validation and merge

## References

- [MIR Aggregate-First Design](../mir_aggregate_first.md)
- [MIR Migration Guide](../../MIGRATION_GUIDE.md)
- [Pipeline Documentation](../../PASSES.md)

---

_This master plan coordinates the implementation of all aggregate lowering
fixes. Each linked task document contains detailed implementation instructions._
