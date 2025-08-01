# Codegen Development Session Plan

## Current State Assessment

The codegen crate successfully translates MIR to CASM for basic language
features including arithmetic, control flow, functions, and variables. However,
there are significant opportunities for optimization and missing features that
need implementation.

## VM Instruction Usage Reference

The Cairo-M VM current supports 32 instructions (opcodes 0-31). Here's the
mapping of which instructions are used by the codegen crate for various
operations:

### Currently Used Instructions

1. **Arithmetic Operations**
   - `STORE_ADD_FP_FP` (opcode 0): Addition between two fp-relative values
   - `STORE_ADD_FP_IMM` (opcode 1): Addition with immediate value
   - `STORE_SUB_FP_FP` (opcode 2): Subtraction between two fp-relative values
   - `STORE_SUB_FP_IMM` (opcode 3): Subtraction with immediate value
   - `STORE_MUL_FP_FP` (opcode 7): Multiplication between two fp-relative values
   - `STORE_MUL_FP_IMM` (opcode 8): Multiplication with immediate value
   - `STORE_DIV_FP_FP` (opcode 9): Division between two fp-relative values
   - `STORE_DIV_FP_IMM` (opcode 10): Division with immediate value

2. **Data Movement**
   - `STORE_IMM` (opcode 6): Store immediate value to fp location

3. **Control Flow**
   - `CALL_ABS_IMM` (opcode 12): Call function at absolute address (label)
   - `RET` (opcode 15): Return from function
   - `JMP_ABS_IMM` (opcode 20): Unconditional jump to absolute address (label)
   - `JNZ_FP_IMM` (opcode 31): Conditional jump if fp value is non-zero

### Instructions Needed for Missing Features

1. **For Load/Store Operations (TODO)**
   - `STORE_DOUBLE_DEREF_FP` (opcode 5): For dereferencing pointers
     `[fp + off2] = [[fp + off0] + off1]`
     - MIR: `InstructionKind::Load { dest, address }` - implements `*ptr`
       dereferencing
     - MIR: `InstructionKind::Store { address, value }` - for indirect stores
       through pointers
     - Example: `let x = *ptr;` or `*ptr = value;`

2. **For Dynamic Function Calls**
   - `CALL_ABS_FP` (opcode 11): Call function at address stored in fp location
     - MIR: `InstructionKind::Call` when the callee is a function pointer
       variable
     - Example: `let fn_ptr = get_function(); fn_ptr(args);`
   - `CALL_REL_FP` (opcode 13): Relative call using fp value
     - MIR: For position-independent code or computed call offsets
   - `CALL_REL_IMM` (opcode 14): Relative call with immediate offset
     - MIR: Alternative to absolute calls for local functions

3. **For Advanced Control Flow**
   - `JMP_ABS_DEREF_FP` (opcode 18): Jump to address stored at fp location
     - MIR: For computed gotos or jump tables (future optimization)
     - MIR: Switch statements with jump table implementation
   - `JMP_REL_IMM` (opcode 27): Relative jump with immediate offset
     - MIR: Alternative to absolute jumps for local branches
   - `JNZ_FP_FP` (opcode 30): Conditional jump with fp-relative offset
     - MIR: Complex conditional branches with computed targets

4. **For Address Computation**
   - `JMP_ABS_ADD_FP_FP` (opcode 16): Jump to computed address (sum of two
     values)
     - MIR: `InstructionKind::GetElementPtr { dest, base, offset }` for
       array/struct access
     - Example: `arr[i]` where both base and index are variables
   - `JMP_ABS_ADD_FP_IMM` (opcode 17): Jump to fp value plus immediate
     - MIR: `InstructionKind::GetElementPtr` with constant offset
     - Example: `struct.field` or `arr[5]`
   - Note: These jump instructions could be repurposed for address arithmetic

5. **For Address-Of Operations**
   - No specific VM instruction needed
     - MIR: `InstructionKind::AddressOf { dest, target }`
     - Implementation: Can use existing arithmetic to compute fp-relative
       addresses
     - Example: `let ptr = &variable;`

### Unused Instructions

The following instructions are not currently used and may not be needed:

- `JMP_ABS_DOUBLE_DEREF_FP` (opcode 19)
- `JMP_ABS_MUL_FP_FP` (opcode 21)
- `JMP_ABS_MUL_FP_IMM` (opcode 22)
- `JMP_REL_ADD_FP_FP` (opcode 23)
- `JMP_REL_ADD_FP_IMM` (opcode 24)
- `JMP_REL_DEREF_FP` (opcode 25)
- `JMP_REL_DOUBLE_DEREF_FP` (opcode 26)
- `JMP_REL_MUL_FP_FP` (opcode 28)
- `JMP_REL_MUL_FP_IMM` (opcode 29)

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
  - Example: In fib function, eliminate the intermediate copy:
    `[fp +4] = [fp +6]` followed by `[fp -3] = [fp +4]`

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
