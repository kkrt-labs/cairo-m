# MIR Crate Implementation Tasks

This directory contains detailed implementation tasks derived from the MIR crate
audit report. Each task has been analyzed against the current codebase and
includes specific implementation guidance.

## Task Organization

Tasks are numbered and prioritized as follows:

- **001-004**: CRITICAL - Correctness and functionality bugs that must be fixed
  immediately
- **005-007**: HIGH - Important issues affecting optimization effectiveness and
  developer experience
- **008-011**: MEDIUM - Code quality and maintainability improvements
- **012**: LOW - Quality of life improvements
- **013**: HIGH - Strategic architectural changes for backend pluggability

## Critical Priority Tasks (Fix First)

1. **[001-critical-pre-opt-dead-stores.md](001-critical-pre-opt-dead-stores.md)**
   - Fix missing dead store elimination call in pre-optimization pass
   - Currently commented out due to GEP aliasing concerns
   - Impacts: Performance optimization effectiveness

2. **[002-critical-ssa-destruction-parallel-copy.md](002-critical-ssa-destruction-parallel-copy.md)**
   - Implement parallel copy semantics in SSA destruction
   - Current sequential assignments can cause incorrect program execution
   - Impacts: Compiler correctness

3. **[003-critical-sroa-typed-gep-mismatch.md](003-critical-sroa-typed-gep-mismatch.md)**
   - Fix mismatch between SROA expectations and lowering output
   - SROA currently no-ops because lowering doesn't emit typed GEPs
   - Impacts: Major optimization pass completely disabled

4. **[004-critical-struct-literal-type-safety.md](004-critical-struct-literal-type-safety.md)**
   - Fix silent type fallback to felt in struct literal lowering
   - Can hide type errors and cause memory corruption
   - Impacts: Type safety guarantees

## High Priority Tasks

5. **[005-high-mem2reg-restrictions-and-offsets.md](005-high-mem2reg-restrictions-and-offsets.md)**
   - Fix overly restrictive promotability rules (U32 not promoted)
   - Fix offset handling bugs in phi source tracking
   - Impacts: Optimization effectiveness and correctness

6. **[006-high-basic-block-naming-api.md](006-high-basic-block-naming-api.md)**
   - Fix misleading API that accepts but ignores block names
   - Add name field to BasicBlock for better debugging
   - Impacts: Developer experience and debugging

7. **[007-high-validation-post-ssa-warnings.md](007-high-validation-post-ssa-warnings.md)**
   - Fix false warnings after SSA destruction
   - Make validation context-aware of compilation phase
   - Impacts: Compiler output clarity

## Medium Priority Tasks

8. **[008-medium-semantic-type-lookup-duplication.md](008-medium-semantic-type-lookup-duplication.md)**
   - Unify semantic type lookups using existing helper methods
   - Eliminate 19+ instances of code duplication
   - Impacts: Code maintainability

9. **[009-medium-memory-builder-helpers.md](009-medium-memory-builder-helpers.md)**
   - Extract common memory access patterns into helpers
   - Reduce 20+ instances of address/load/store sequences
   - Impacts: Code clarity and consistency

10. **[010-medium-builder-api-naming.md](010-medium-builder-api-naming.md)**
    - Standardize inconsistent builder API naming conventions
    - Remove confusing suffixes like \_auto, \_with_dest
    - Impacts: API usability

11. **[011-medium-unused-ir-operations.md](011-medium-unused-ir-operations.md)**
    - Remove unused SSA aggregate instructions
    - Clean up never-emitted instruction kinds
    - Impacts: Code complexity reduction

## Low Priority Tasks

12. **[012-low-quality-of-life-improvements.md](012-low-quality-of-life-improvements.md)**
    - Various small improvements (logging, documentation, etc.)
    - Replace eprintln with proper logging
    - Impacts: Developer experience

## Strategic Tasks

13. **[013-high-backend-pluggability.md](013-high-backend-pluggability.md)**
    - Implement backend-pluggable architecture
    - Enable support for multiple code generation targets
    - Impacts: Future extensibility and ecosystem growth

## Implementation Order Recommendation

1. **Phase 1 - Critical Fixes** (Tasks 1-4)
   - Fix correctness issues first
   - Enable disabled optimizations

2. **Phase 2 - High Priority** (Tasks 5-7, 13)
   - Improve optimization effectiveness
   - Fix developer experience issues
   - Implement backend pluggability

3. **Phase 3 - Code Quality** (Tasks 8-11)
   - Reduce duplication
   - Improve API consistency
   - Remove dead code

4. **Phase 4 - Polish** (Task 12)
   - Quality of life improvements
   - Documentation enhancements

## Notes

- Each task document follows a consistent format: Why, What, How
- All tasks have been verified against the current codebase state
- Implementation details include specific line numbers and code examples
- Testing strategies are provided for each task
