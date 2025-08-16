# Task 015: Low-Priority Documentation - MIR Aggregate-First Design

## Why

Comprehensive documentation is essential for the new aggregate-first MIR design
because:

1. **Design Preservation**: The refactoring from memory-centric to value-based
   aggregates represents a fundamental architectural shift that must be properly
   documented to prevent future regressions to memory-first style
2. **Contributor Onboarding**: New team members need clear guidance on the
   aggregate-first approach to avoid implementing features using the old
   memory-based patterns
3. **Maintenance Clarity**: The transition from complex SROA/Mem2Reg
   optimization passes to simpler value-based operations requires clear
   documentation of the design rationale and implementation approach
4. **Migration Support**: Contributors working on related code need guidance on
   how the new system works and how to migrate existing patterns

## What

Create comprehensive documentation for the aggregate-first MIR design including:

### Core Documentation Requirements

1. **Design Document (`mir_aggregate_first.md`)**:
   - High-level design overview and rationale
   - Comparison with the old memory-centric approach
   - New instruction types and their semantics
   - Optimization pipeline changes

2. **Before/After Examples**:
   - MIR dumps showing old vs new lowering patterns
   - Code snippets demonstrating the transformation
   - Performance and complexity improvements

3. **Migration Guide**:
   - How to identify and update memory-based patterns
   - Guidelines for implementing new aggregate features
   - Common pitfalls and best practices

4. **Integration Documentation**:
   - Updates to existing developer guides
   - Links from README and development documentation
   - API documentation for new instruction builders

## How

### Implementation Steps

#### 1. Create Core Design Document

**File**: `docs/mir_aggregate_first.md`

**Structure**:

```markdown
# MIR Aggregate-First Design

## Overview

- Problem with memory-centric approach
- Benefits of value-based aggregates
- High-level design principles

## New Instructions

- MakeTuple/ExtractTuple semantics
- MakeStruct/ExtractStructField operations
- InsertField for mutations

## Lowering Changes

- Tuple/struct literals as SSA values
- Field access via extract operations
- Assignment as SSA rebinding

## Optimization Pipeline

- Removal of SROA/Mem2Reg complexity
- Simple constant folding opportunities
- Variable-SSA pass for control flow

## Backend Integration

- Late aggregate lowering for ABI compatibility
- Feature flags for rollout
```

#### 2. Add Before/After Examples

**Include in design document**:

- Simple tuple creation and access
- Struct literal and field access
- Assignment and control flow merging
- Function calls returning aggregates

**Example snippets**:

```rust
// Before (memory-centric):
%0 = framealloc Point
%1 = get_element_ptr %0, 0
store %1, %x
%2 = get_element_ptr %0, 1
store %2, %y
%3 = load %0

// After (value-based):
%0 = makestruct { x: %x, y: %y }
```

#### 3. Create Migration Guide

**File**: `docs/mir_migration_guide.md`

**Content**:

- Identifying legacy memory patterns
- Converting to aggregate instructions
- Updating optimization passes
- Testing strategies

#### 4. Update Existing Documentation

**Files to update**:

- `README.md`: Link to new design docs
- `CLAUDE.md`: Update MIR pipeline description
- `crates/compiler/mir/README.md`: Reference new instructions

**Integration points**:

- Add links in development workflow documentation
- Update API documentation for instruction builders
- Include in code review guidelines

#### 5. Add Inline Documentation

**Code documentation**:

- Comprehensive rustdoc for new instruction types
- Builder API documentation with examples
- Pass documentation explaining the new pipeline

#### 6. Create Examples and Tests

**Example programs**:

- Simple aggregate usage patterns
- Complex control flow scenarios
- Performance comparison cases

**Test documentation**:

- How to verify correct lowering
- Snapshot test expectations
- Performance regression testing

### Deliverables

1. **`docs/mir_aggregate_first.md`** - Core design document
2. **`docs/mir_migration_guide.md`** - Contributor migration guide
3. **Updated README.md** - Links to new documentation
4. **Updated CLAUDE.md** - Pipeline description updates
5. **Inline rustdoc** - API documentation for new instructions
6. **Example snippets** - Before/after MIR comparisons

### Validation Criteria

- [ ] Design document reviewed by 1-2 teammates
- [ ] All new instruction types have comprehensive rustdoc
- [ ] Migration guide includes concrete examples
- [ ] Documentation linked from main development guides
- [ ] Examples compile and demonstrate key concepts
- [ ] No regressions to memory-first patterns in new code

**Priority**: LOW **Dependencies**: Most tasks (documents the complete system
after refactoring) **Estimated Effort**: 1-2 days for comprehensive
documentation

### Success Metrics

- New contributors can understand aggregate-first design from documentation
  alone
- No new code uses deprecated memory-based aggregate patterns
- Documentation prevents design regressions during future development
- Clear path for extending aggregate support to arrays and other types
