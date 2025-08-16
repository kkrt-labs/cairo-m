# Task: Remove or Repurpose Unused IR Operations

## Priority

MEDIUM - COMPLETED

## Why

The MIR crate contains several instruction kinds that are never emitted by the
lowering phase, creating maintenance burden and code complexity:

- **Code bloat**: Unused instruction variants add complexity to pattern matching
  and analysis code throughout the compiler
- **Testing overhead**: Each unused instruction requires test coverage and
  maintenance despite never being generated
- **API confusion**: Having unused operations in the public API misleads
  developers about which instructions are actually supported
- **Codegen inconsistency**: The code generator explicitly marks these
  operations as "not yet supported", creating false expectations
- **Maintenance debt**: Changes to instruction handling must account for unused
  variants that will never be exercised in practice

## What

### Unused Instructions Identified

The following instruction kinds are defined but never emitted by the IR
generation phase:

**SSA Aggregate Operations**
(`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/instruction.rs`):

- `BuildStruct` - Creates struct values from field values without memory
  allocation
- `BuildTuple` - Creates tuple values from element values without memory
  allocation
- `ExtractValue` - Extracts fields/elements from aggregate values without memory
  access
- `InsertValue` - Creates new aggregate with one field/element replaced
- `GetElementPtrTyped` - Type-safe pointer arithmetic using field paths instead
  of integer offsets

### Current State Analysis

**Lowering Phase**
(`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/ir_generation.rs`):

- Never emits any of the identified unused instructions
- Uses memory-based approach with `FrameAlloc`, `GetElementPtr`, `Load`, and
  `Store`

**Code Generator**
(`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/codegen/src/generator.rs`):

- Explicitly returns "not yet supported" errors for `BuildStruct`, `BuildTuple`,
  `ExtractValue`, `InsertValue`
- Has partial implementation for `GetElementPtrTyped` but it's never reached

**SROA Pass**
(`/Users/msaug/kkrt-labs/cairo-m/crates/compiler/mir/src/passes/sroa.rs`):

- Designed to work with these SSA aggregate operations
- Currently only processes memory-based aggregates due to missing SSA aggregate
  generation
- Contains comprehensive test cases that manually construct these unused
  instructions

## How

### Option A: Remove Unused Instructions (Recommended)

**Rationale**: These instructions add complexity without providing value. The
current memory-based approach works and is well-tested.

**Migration Steps**:

1. **Remove instruction variants** from `InstructionKind` enum:
   - Remove `BuildStruct`, `BuildTuple`, `ExtractValue`, `InsertValue`,
     `GetElementPtrTyped`
   - Remove corresponding constructor methods in `Instruction` impl

2. **Update pattern matching** throughout codebase:
   - Remove branches from `destinations()`, `used_values()`, `validate()`,
     `pretty_print()`
   - Remove codegen error handling for these instructions
   - Update any exhaustive matches in analysis passes

3. **Simplify SROA pass**:
   - Remove SSA aggregate scalarization logic (`process_ssa_aggregates()`)
   - Remove `SsaAggregateInfo` struct and related methods
   - Focus SROA exclusively on memory-based aggregate splitting
   - Remove or rewrite SROA tests that use these instructions

4. **Update documentation**:
   - Remove references to SSA aggregate operations from comments and docs
   - Update SROA documentation to reflect memory-only approach

### Option B: Adopt SSA Aggregate Instructions (Not Recommended)

**Rationale**: While these instructions could enable more sophisticated
optimizations, they require significant implementation effort without clear
benefit over the current approach.

**Requirements if pursued**:

- Modify IR generation to emit SSA aggregate instructions for struct/tuple
  construction
- Complete codegen implementation for all SSA aggregate operations
- Ensure type system properly handles aggregate values in SSA form
- Add comprehensive test coverage for the SSA aggregate path

## Testing

### For Option A (Remove Instructions):

1. **Compilation verification**: Ensure all pattern matches compile after
   removing instruction variants
2. **SROA functionality**: Verify SROA still works correctly with only
   memory-based aggregates
3. **Regression testing**: Run full test suite to ensure no functionality is
   lost
4. **API consistency**: Verify instruction-related APIs work correctly with
   reduced instruction set

### For Option B (Adopt Instructions):

1. **IR generation tests**: Verify SSA aggregate instructions are correctly
   emitted
2. **Codegen tests**: Verify all SSA aggregate operations generate correct CASM
3. **SROA integration**: Test that SROA correctly handles both memory and SSA
   aggregates
4. **Type safety**: Verify aggregate types are preserved correctly through the
   pipeline

## Implementation Summary

Successfully removed unused IR operations from the codebase:

1. **Removed from InstructionKind enum**:
   - `BuildStruct` - SSA aggregate construction
   - `BuildTuple` - SSA aggregate construction
   - `ExtractValue` - SSA aggregate field extraction
   - `InsertValue` - SSA aggregate field insertion
   - `GetElementPtrTyped` - Type-safe pointer arithmetic

2. **Updated SROA pass**:
   - Removed `process_ssa_aggregates()` method
   - Removed `SsaAggregateInfo` struct
   - Simplified to focus exclusively on memory-based aggregates
   - Updated to work without GetElementPtrTyped (though optimization is now
     limited)

3. **Updated codegen**:
   - Removed cases for all unused instructions
   - Added comment explaining their removal

4. **Updated tests**:
   - Rewrote SROA tests to work without the removed instructions
   - Added explanatory comments about the limitations

The compiler now uses a simpler, memory-based approach for aggregates
exclusively, reducing code complexity and maintenance burden.

## Impact

### Option A (Remove - Recommended):

**Positive Impacts**:

- **Reduced complexity**: Simpler instruction set with fewer variants to handle
- **Clearer API**: Only supported operations are exposed in the public API
- **Faster compilation**: Less pattern matching overhead in hot paths
- **Focused testing**: Test resources concentrated on actually-used
  functionality
- **Consistent expectations**: Codegen supports all defined instructions

**Breaking Changes**:

- **SROA API changes**: Methods and types related to SSA aggregates removed
- **Pattern match updates**: Code matching on instruction kinds needs updates
- **Test updates**: Manual test cases using these instructions need rewriting

**Estimated Effort**:

- **Implementation**: 1 day for removing instructions and updating matches
- **SROA simplification**: 1 day for removing SSA aggregate logic
- **Testing**: 1 day for validation and test updates
- **Documentation**: 0.5 days for comment and doc updates

### Option B (Adopt - Not Recommended):

**Positive Impacts**:

- **Advanced optimizations**: Enable more sophisticated aggregate optimizations
- **Memory efficiency**: Potential for better aggregate handling without memory
  allocation

**Negative Impacts**:

- **Implementation complexity**: Significant work required across lowering,
  codegen, and optimization passes
- **Maintenance burden**: Two different aggregate handling approaches to
  maintain
- **Testing complexity**: Comprehensive test coverage for underutilized
  functionality

**Estimated Effort**: 1-2 weeks of implementation across multiple compiler
phases
