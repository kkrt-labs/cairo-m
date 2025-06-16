# Codegen Development Session Plan

## Current State Assessment

The codegen crate successfully translates MIR to CASM for basic language
features including arithmetic, control flow, functions, and variables. However,
there are significant opportunities for optimization and missing features that
need implementation.

## High Priority Tasks

### 1. Implement Missing MIR Instructions

**Context**: Several MIR instructions are marked with `todo!()` and need
implementation for full language support.

- [ ] **Implement Load/Store instructions**

  - Add support for indirect memory access through pointers
  - Update `builder.rs::generate_instruction()` to handle `MirInstruction::Load`
    and `MirInstruction::Store`
  - Test with pointer dereference scenarios

- [ ] **Implement GetElementPtr**

  - Add struct field access support
  - Add array indexing support
  - Calculate proper offsets based on type information
  - Test with nested struct access and array operations

- [ ] **Implement AddressOf operation**

  - Generate code to compute addresses of values
  - Handle both stack variables and heap allocations
  - Test with pointer creation scenarios

- [ ] **Implement Cast operations**
  - Add type conversion support (if needed by Cairo VM)
  - Handle numeric type conversions
  - Test with mixed-type expressions

### 2. Code Generation Optimizations

**Context**: Current codegen produces correct but inefficient CASM. Analysis
shows multiple optimization opportunities.

- [ ] **Eliminate redundant copies**

  - Implement copy propagation in `generator.rs`
  - Track value movement to avoid unnecessary copies
  - Example: In fib function, eliminate the intermediate copy: `[fp+4] = [fp+6]`
    followed by `[fp-3] = [fp+4]`

- [ ] **Remove dead code after returns**

  - Track unreachable code in basic blocks
  - Skip code generation for instructions after `return` in the same block
  - Add a reachability analysis pass

- [ ] **Implement constant folding**

  - Reviewer note: should this instead be done at the MIR level? Doing it at the
    codegen level is not very efficient as it could be done upstream.
  - Detect compile-time constant expressions
  - Evaluate them during code generation
  - Example: `10 + 32` should generate `6 -3 42` instead of runtime addition

- [x] **Optimize control flow**
  - Remove jumps to immediately following blocks (fall-through) ✓
  - Merge consecutive blocks when possible (still TODO)
  - Example: Remove `jump abs label` when `label:` is the next instruction ✓
  - **Implemented**: Added `generate_terminator` that detects when jump targets
    are the immediately following block and skips generating unnecessary jumps.
    This optimization reduces instruction count in if-else statements and other
    control flow constructs.

### 3. Clean Up Generated Output

- [ ] **Fix duplicate label generation**

  - Investigate why functions generate duplicate labels (e.g., `main:` appears
    twice)
  - Update `generator.rs` to emit each label only once
  - Ensure label uniqueness

- [ ] **Improve code formatting**
  - Add proper indentation for instructions within labels
  - Align opcodes and operands for readability
  - Add meaningful comments for complex operations

### 4. MIR-Level Optimizations (Pre-codegen)

**Context**: Some optimizations are better performed at the MIR level before
code generation.

- [ ] **Add MIR optimization pass**

  - Create new module in `mir/src/passes.rs`
  - Implement dead store elimination
  - Implement copy propagation at MIR level
  - Run before codegen in the compilation pipeline

- [ ] **Optimize stackalloc usage**
  - Analyze MIR patterns like `stackalloc` followed by immediate `store` and
    `return`
  - Eliminate unnecessary allocations when values can be used directly
  - Update MIR generation to produce more efficient patterns

### 5. Advanced Features

- [ ] **Function inlining**

  - Identify small functions suitable for inlining
  - Implement inlining transformation at MIR or codegen level
  - Add heuristics for when to inline (size threshold, call frequency)

- [ ] **Tail call optimization**

  - Detect tail calls in MIR
  - Generate optimized code that reuses the current stack frame
  - Test with recursive functions like factorial

- [ ] **Better error messages**
  - Add source location tracking through codegen
  - Improve error context in `CodegenError`
  - Add suggestions for common issues

### 6. Testing and Documentation

- [ ] **Add comprehensive test coverage**

  - Add tests for edge cases (empty functions, no-return functions)
  - Add tests for complex control flow patterns
  - Add tests for optimization passes

- [ ] **Document calling convention**

  - Create detailed documentation of the Cairo calling convention
  - Add diagrams showing stack layout
  - Document any deviations or compiler-specific decisions

- [ ] **Add performance benchmarks**
  - Create benchmark suite for generated code
  - Measure instruction count for common patterns
  - Track optimization improvements

## Implementation Order

1. **Start with missing instructions** (Load/Store, GetElementPtr) as they block
   language features
2. **Then focus on easy optimizations** (redundant copies, dead code
   elimination)
3. **Move to MIR-level optimizations** for broader impact
4. **Finally tackle advanced features** like inlining and tail calls

## Getting Started

To begin work on any task:

1. Check out the codebase and run existing tests:
   `cargo test --package cairo-m-compiler-codegen`
2. Pick a task from the list above
3. Write tests first (add `.cm` files in `tests/test_cases/`)
4. Implement the feature/optimization
5. Run tests and update snapshots: `cargo insta review`
6. Ensure all tests pass before moving to the next task

## Notes for Contributors

- The codegen crate is the final stage - ensure generated CASM is valid
- Always test with the Cairo VM to verify correctness
- Performance matters - every instruction counts in blockchain context
- Maintain backward compatibility with existing test cases
- Document any changes to the calling convention or stack layout
