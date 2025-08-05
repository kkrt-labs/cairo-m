# Cairo-M MIR Lowering Refactoring Plan

## Executive Summary

This document outlines a comprehensive refactoring plan for the Cairo-M
compiler's MIR (Middle Intermediate Representation) lowering module. The
refactoring addresses critical architectural issues while preserving all
existing functionality through incremental, testable changes.

### Goals

- Reduce `MirBuilder` from 800+ lines to under 200 lines
- Separate concerns: CFG management, instruction generation, optimization,
  semantic resolution
- Enable easy addition of new language features (switch statements, pattern
  matching)
- Improve compilation performance by 15-20% through caching and better data flow
- Maintain 100% backward compatibility and test coverage

### Risk Assessment

- **Overall Risk**: LOW-MEDIUM
- **Mitigation**: Each phase is independently testable with rollback capability
- **Timeline**: 4-6 weeks for complete implementation
- **Team Size**: 1-2 developers

---

## Current State Analysis

### Pain Points Identified

#### 1. Monolithic MirBuilder (crates/compiler/mir/src/lowering/builder.rs)

- **Lines**: 226+
- **Responsibilities**: 7+ mixed concerns
  - CFG construction
  - Instruction generation
  - Semantic type resolution
  - Definition mapping
  - Loop context management
  - Function resolution
  - Binary op conversion

#### 2. Complex Statement Lowering (crates/compiler/mir/src/lowering/stmt.rs)

- **Lines**: 865 total
- **`lower_let_statement`**: Lines 58-167 (109 lines)
  - Special case handling: Lines 79-83
  - Binary op optimization: Lines 84-133
  - Aggregate literal handling: Lines 138-162
  - Generic pattern binding: Lines 164-166
- **`try_lower_let_special_case`**: Lines 679-768 (89 lines)
  - Tuple destructuring optimization
  - Function call tuple unpacking

#### 3. Repetitive Binary Op Conversion (crates/compiler/mir/src/lowering/builder.rs)

- **Lines**: 127-199 (72 lines)
- **Pattern**: 30+ nearly identical match arms
- **Types**: Felt, U32, Bool operations

#### 4. Scattered Optimizations

- Fast paths mixed with lowering logic
- Dead code elimination in lowering phase
- Binary op fusion during statement processing
- Tuple destructuring optimizations inline

### Features to Preserve

1. **Optimizations**
   - Direct tuple destructuring: `let (a, b) = (x, y)`
   - Function call unpacking: `let (a, b) = func()`
   - Binary operation fusion: `let x = a + b` → single instruction
   - Aggregate literal handling: Direct address passing for structs/tuples
   - Dead variable elimination: Skip storage for unused variables

2. **Control Flow**
   - Loop stack management for break/continue
   - Block termination tracking
   - Multi-level loop nesting support

3. **Type System Integration**
   - Semantic type caching
   - U32 vs Felt operation selection
   - Tuple element type tracking

---

## Phase 0: Foundation Layer (Week 1, Days 1-2)

### 0.1 Introduce Place/Value Type Safety

**Objective**: Add compile-time distinction between l-values and r-values

**File**: `crates/compiler/mir/src/value.rs`

**Add after line 45**:

```rust
/// Represents a memory location (l-value) that can be stored to
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Place(pub ValueId);

impl Place {
    pub fn new(id: ValueId) -> Self {
        Place(id)
    }

    pub fn value_id(self) -> ValueId {
        self.0
    }
}

impl From<ValueId> for Place {
    fn from(id: ValueId) -> Self {
        Place(id)
    }
}

impl From<Place> for Value {
    fn from(place: Place) -> Self {
        Value::Operand(place.0)
    }
}
```

**Update signatures**:

- `Instruction::store(dest: Place, value: Value)`
- `MirType::emit_store(&self, builder: &mut MirBuilder, dest: Place, value: Value)`
- `MirType::emit_load(&self, builder: &mut MirBuilder, src: Place) -> Value`

**Testing**: Run existing tests to ensure compatibility

```bash
cargo test -p cairo-m-compiler-mir
```

### 0.2 Extract InstrBuilder

**Objective**: Centralize instruction creation with fluent API

**Create file**: `crates/compiler/mir/src/builder/instr_builder.rs`

```rust
use crate::{BinaryOp, Instruction, MirFunction, Value, ValueId, Literal, MirType};

pub struct InstrBuilder<'f> {
    function: &'f mut MirFunction,
}

impl<'f> InstrBuilder<'f> {
    pub fn new(function: &'f mut MirFunction) -> Self {
        Self { function }
    }

    /// Create a binary operation, returning the destination
    pub fn binary(&mut self, op: BinaryOp, lhs: Value, rhs: Value, ty: MirType) -> ValueId {
        let dest = self.function.new_typed_value_id(ty);
        Instruction::binary_op(op, dest, lhs, rhs)
        dest
    }

    /// Load from a memory location
    pub fn load(&mut self, src: Place, ty: MirType) -> ValueId {
        let dest = self.function.new_typed_value_id(ty);
        Instruction::load(dest, Value::Operand(src.value_id()))
        dest
    }

    /// Store to a memory location (returns nothing)
    pub fn store(&mut self, dest: Place, value: Value) -> Instruction {
        Instruction::store(dest.value_id(), value)
    }

    /// Allocate stack space
    pub fn stack_alloc(&mut self, size: u32, ty: MirType) -> ValueId {
        let dest = self.function.new_typed_value_id(MirType::pointer(ty));
        Instruction::stack_alloc(dest, size)
        dest
    }

    /// Create a literal value
    pub fn literal(&mut self, lit: Literal, ty: MirType) -> ValueId {
        let dest = self.function.new_typed_value_id(ty);
        Instruction::load_literal(dest, lit)
        dest
    }
}
```

**Migration locations**:

- `lowering/expr.rs:303-305`: Replace with `self.instr().binary()`
- `lowering/stmt.rs:122-127`: Use builder pattern
- `lowering/stmt.rs:320-335`: Binary op generation

**Verification**: Compile and run single test

```bash
cargo test -p cairo-m-compiler-mir test_simple_function
```

---

## Phase 1: State Separation (Week 1, Days 3-4)

### 1.1 Split MirBuilder State

**Objective**: Separate immutable context from mutable state

**File**: `crates/compiler/mir/src/lowering/builder.rs`

**Replace lines 27-51 with**:

```rust
/// Immutable compilation context shared across lowering
pub struct LoweringContext<'a, 'db> {
    pub db: &'db dyn SemanticDb,
    pub file: File,
    pub crate_id: Crate,
    pub semantic_index: &'a SemanticIndex,
    pub function_mapping: &'a FxHashMap<DefinitionId<'db>, (&'a Definition, FunctionId)>,
    pub file_id: u64,

    // Caches to improve performance
    pub expr_type_cache: RefCell<FxHashMap<ExpressionId, MirType>>,
    pub definition_cache: RefCell<FxHashMap<FileScopeId, Vec<(DefinitionId, Definition)>>>,
}

/// Mutable state for the function being built
pub struct MirState {
    pub mir_function: MirFunction,
    pub current_block_id: BasicBlockId,
    pub definition_to_value: FxHashMap<MirDefinitionId, ValueId>,
    pub function_def_id: Option<DefinitionId>,
    pub is_terminated: bool,
    pub loop_stack: Vec<(BasicBlockId, BasicBlockId)>,
}

/// Main builder combining context and state
pub struct MirBuilder<'a, 'db> {
    pub ctx: LoweringContext<'a, 'db>,
    pub state: MirState,
    instr_builder: InstrBuilder<'a>,
}

impl<'a, 'db> LoweringContext<'a, 'db> {
    /// Get or compute the MIR type for an expression
    pub fn get_expr_type(&self, expr_id: ExpressionId) -> MirType {
        let mut cache = self.expr_type_cache.borrow_mut();
        cache.entry(expr_id)
            .or_insert_with(|| {
                let sem_type = expression_semantic_type(
                    self.db,
                    self.crate_id,
                    self.file,
                    expr_id,
                    None
                );
                MirType::from_semantic_type(self.db, sem_type)
            })
            .clone()
    }
}
```

**Update all methods to use new structure**:

- Replace `self.db` with `self.ctx.db`
- Replace `self.ctx.semantic_index` with `self.ctx.semantic_index`
- Replace `self.mir_function` with `self.state.mir_function`
- Replace `self.current_block_id` with `self.state.current_block_id`

### 1.2 Create CfgBuilder

**Objective**: Extract control flow graph operations

**Create file**: `crates/compiler/mir/src/builder/cfg_builder.rs`

```rust
use crate::{BasicBlockId, MirFunction, Terminator, BasicBlock};

pub struct CfgBuilder<'s> {
    state: &'s mut MirState,
}

impl<'s> CfgBuilder<'s> {
    pub fn new(state: &'s mut MirState) -> Self {
        Self { state }
    }

    /// Create a new basic block
    pub fn new_block(&mut self, name: impl Into<String>) -> BasicBlockId {
        self.state.mir_function.new_block(name)
    }

    /// Switch to a different block for instruction emission
    pub fn switch_to_block(&mut self, block_id: BasicBlockId) {
        self.state.current_block_id = block_id;
        self.state.is_terminated = false;
    }

    /// Terminate the current block
    pub fn terminate(&mut self, terminator: Terminator) {
        let block = self.state.mir_function.basic_blocks
            .get_mut(self.state.current_block_id)
            .expect("Current block should exist");
        block.set_terminator(terminator);
        self.state.is_terminated = true;
    }

    /// Get the current block for inspection
    pub fn current_block(&self) -> &BasicBlock {
        self.state.mir_function.basic_blocks
            .get(self.state.current_block_id)
            .expect("Current block should exist")
    }

    /// Check if current block is terminated
    pub fn is_terminated(&self) -> bool {
        self.state.is_terminated
    }
}
```

**Migration locations**:

- `lowering/control_flow.rs`: All block operations
- `lowering/stmt.rs:390-455`: If statement block management
- `lowering/stmt.rs:456-510`: While loop blocks
- `lowering/stmt.rs:511-559`: Loop blocks
- `lowering/stmt.rs:560-635`: For loop blocks

---

## Phase 1.3: Complete Builder Integration (Immediate)

**Goal**: Fully transition to using the new builder APIs throughout the
codebase, eliminating all direct manipulation of blocks, instructions, and
state.

### Changes Required:

1. **Replace Direct Block Creation**
   - Change all `self.state.mir_function.add_basic_block()` to
     `self.new_block()`
   - Use CfgBuilder's convenience methods for control flow patterns

2. **Replace Direct State Mutations**
   - Change all `self.state.current_block_id = ...` to
     `self.switch_to_block(...)`
   - Change all `self.state.is_terminated = ...` to use CfgBuilder state
     management

3. **Implement InstrBuilder Usage**
   - Add `instr_builder()` method to MirBuilder
   - Replace all direct `Instruction::*` calls with InstrBuilder methods
   - Replace `self.add_instruction()` with InstrBuilder fluent API

4. **Use CfgBuilder Convenience Methods**
   - Replace manual if-then-else block creation with `create_if_blocks()`
   - Replace manual loop block creation with `create_loop_blocks()`
   - Replace manual for-loop block creation with `create_for_loop_blocks()`

5. **Cleanup**
   - Remove backup files (`.bak`, `.bak2`, `.bak3`)
   - Remove or implement unused `definition_cache`
   - Fix all direct terminator manipulations

### Implementation Order:

1. Add `instr_builder()` method to MirBuilder
2. Refactor `stmt.rs` to use CfgBuilder for all block operations
3. Refactor `expr.rs` to use InstrBuilder for all instructions
4. Refactor `utils.rs` to use InstrBuilder
5. Clean up backup files and unused code
6. Run tests and fix any issues

### Files to Modify:

- `lowering/builder.rs` - Add instr_builder() method
- `lowering/stmt.rs` - Use CfgBuilder for all control flow
- `lowering/expr.rs` - Use InstrBuilder for all instructions
- `lowering/utils.rs` - Use InstrBuilder for call instructions
- `lowering/control_flow.rs` - Ensure consistency with new APIs

---

## Phase 2: Extract Optimizations (Week 1, Day 5 - Week 2, Day 2)

### 2.1 Create Pre-optimization Pass

**Objective**: Move all fast-path optimizations to a separate pass

**Create file**: `crates/compiler/mir/src/passes/pre_opt.rs`

```rust
use crate::{MirFunction, MirPass, Instruction, InstructionKind, Value, Pattern};
use rustc_hash::FxHashMap;

/// Pre-optimization pass that runs immediately after lowering
/// Handles special-case optimizations that were previously in the lowering phase
pub struct PreOptimizationPass {
    // Track which optimizations were applied for debugging
    optimizations_applied: Vec<String>,
}

impl PreOptimizationPass {
    pub fn new() -> Self {
        Self {
            optimizations_applied: Vec::new(),
        }
    }

    /// Optimize tuple destructuring patterns
    /// Converts: let (a, b) = (x, y) into direct assignments
    fn optimize_tuple_destructuring(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        // Pattern: sequential loads from tuple followed by stores
        // Look for:
        //   %tuple_addr = stack_alloc
        //   store %tuple_addr[0], %x
        //   store %tuple_addr[1], %y
        //   %a = load %tuple_addr[0]
        //   %b = load %tuple_addr[1]
        // Replace with:
        //   %a = %x
        //   %b = %y

        for block in function.basic_blocks.iter_mut() {
            let mut i = 0;
            while i < block.instructions.len() {
                // Detection logic here
                if self.is_tuple_destructure_pattern(&block.instructions[i..]) {
                    self.apply_tuple_destructure_opt(&mut block.instructions, i);
                    modified = true;
                    self.optimizations_applied.push("tuple_destructure".to_string());
                }
                i += 1;
            }
        }

        modified
    }

    /// Optimize binary operations in let statements
    /// Converts: let x = a + b with allocation + store into single instruction
    fn optimize_binary_let(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        for block in function.basic_blocks.iter_mut() {
            let mut i = 0;
            while i < block.instructions.len().saturating_sub(2) {
                // Pattern:
                //   %addr = stack_alloc
                //   %tmp = binary_op op, %a, %b
                //   store %addr, %tmp
                // Replace with:
                //   %addr = stack_alloc
                //   binary_op op, %addr, %a, %b  (direct destination)

                if let InstructionKind::StackAlloc { dest, .. } = &block.instructions[i].kind {
                    let alloc_dest = *dest;

                    if i + 2 < block.instructions.len() {
                        if let InstructionKind::BinaryOp { op, dest, left, right } = &block.instructions[i + 1].kind {
                            let binary_dest = *dest;

                            if let InstructionKind::Store { dest: store_dest, value: Value::Operand(store_val) } = &block.instructions[i + 2].kind {
                                if *store_dest == alloc_dest && *store_val == binary_dest {
                                    // Found the pattern! Optimize it
                                    let new_binary = Instruction::binary_op(*op, alloc_dest, *left, *right);
                                    block.instructions[i + 1] = new_binary;
                                    block.instructions.remove(i + 2);
                                    modified = true;
                                    self.optimizations_applied.push("binary_let_fusion".to_string());
                                }
                            }
                        }
                    }
                }
                i += 1;
            }
        }

        modified
    }

    /// Remove dead stores for unused variables
    fn eliminate_dead_stores(&mut self, function: &mut MirFunction) -> bool {
        let use_counts = function.get_value_use_counts();
        let mut modified = false;

        for block in function.basic_blocks.iter_mut() {
            block.instructions.retain(|instr| {
                if let InstructionKind::Store { dest, .. } = &instr.kind {
                    if use_counts.get(dest).copied().unwrap_or(0) == 0 {
                        modified = true;
                        self.optimizations_applied.push("dead_store_elimination".to_string());
                        return false; // Remove this instruction
                    }
                }
                true
            });
        }

        modified
    }
}

impl MirPass for PreOptimizationPass {
    fn run(&mut self, function: &mut MirFunction) -> bool {
        let mut modified = false;

        // Run all optimization sub-passes
        modified |= self.optimize_tuple_destructuring(function);
        modified |= self.optimize_binary_let(function);
        modified |= self.eliminate_dead_stores(function);

        if !self.optimizations_applied.is_empty() {
            log::debug!("Pre-optimizations applied: {:?}", self.optimizations_applied);
        }

        modified
    }

    fn name(&self) -> &'static str {
        "pre-optimization"
    }
}
```

**Remove from lowering**:

1. **From `lowering/stmt.rs:79-83`** - Remove special case check:

```rust
// DELETE THIS:
if self.try_lower_let_special_case(pattern, value, scope_id)? {
    return Ok(());
}
```

2. **From `lowering/stmt.rs:84-133`** - Remove binary optimization:

```rust
// DELETE THIS ENTIRE BLOCK:
if let Pattern::Identifier(name) = pattern
    && let Expression::BinaryOp { op, left, right } = value.value()
{
    // ... binary operation optimization ...
}
```

3. **From `lowering/stmt.rs:679-768`** - Remove entire
   `try_lower_let_special_case` function

4. **From `lowering/stmt.rs:97-106`** - Remove dead variable handling:

```rust
// DELETE THIS:
let is_used = is_definition_used(self.ctx.semantic_index, def_idx);
if !is_used {
    // ... dummy handling ...
}
```

**New simplified `lower_let_statement`**:

```rust
pub(super) fn lower_let_statement(
    &mut self,
    pattern: &Pattern,
    value: &Spanned<Expression>,
) -> Result<(), String> {
    // Simply lower the expression and bind to pattern
    let rhs_value = self.lower_expression(value)?;
    let scope_id = self.get_expression_scope(value)?;
    self.lower_pattern(pattern, rhs_value, scope_id)?;
    Ok(())
}
```

### 2.2 Update Pass Pipeline

**File**: `crates/compiler/mir/src/db.rs` or wherever passes are run

**Add pre-optimization pass**:

```rust
pub fn optimize_mir(function: &mut MirFunction) {
    // Run pre-optimization first
    let mut pre_opt = PreOptimizationPass::new();
    pre_opt.run(function);

    // Then existing passes
    let mut fuse_cmp = FuseCmpBranch::new();
    fuse_cmp.run(function);

    // Other passes...
}
```

---

## Phase 3: Statement Modularization (Week 2, Days 3-5)

### 3.1 Create Statement Module Structure

**Create directory**: `crates/compiler/mir/src/lowering/stmt/`

**File**: `crates/compiler/mir/src/lowering/stmt/mod.rs`

```rust
mod let_stmt;
mod control;
mod loops;
mod flow;
mod assign;

pub use let_stmt::LetStatementLowerer;
pub use control::IfStatementLowerer;
pub use loops::{WhileLowerer, ForLowerer, LoopLowerer};
pub use flow::{ReturnLowerer, BreakLowerer, ContinueLowerer};
pub use assign::AssignmentLowerer;

use crate::MirBuilder;
use cairo_m_compiler_parser::parser::{Statement, Spanned};

/// Trait for lowering different statement types
pub trait StatementLowerer {
    fn lower(&mut self, builder: &mut MirBuilder) -> Result<(), String>;
}

/// Main dispatch function for statement lowering
pub fn lower_statement(
    builder: &mut MirBuilder,
    stmt: &Spanned<Statement>
) -> Result<(), String> {
    match stmt.value() {
        Statement::Let { pattern, value, .. } => {
            LetStatementLowerer::new(pattern, value).lower(builder)
        }
        Statement::Return { value } => {
            ReturnLowerer::new(value.as_ref()).lower(builder)
        }
        Statement::Assignment { lhs, rhs } => {
            AssignmentLowerer::new(lhs, rhs).lower(builder)
        }
        Statement::If { condition, then_block, else_block } => {
            IfStatementLowerer::new(condition, then_block, else_block.as_deref()).lower(builder)
        }
        Statement::While { condition, body } => {
            WhileLowerer::new(condition, body).lower(builder)
        }
        Statement::Loop { body } => {
            LoopLowerer::new(body).lower(builder)
        }
        Statement::For { init, condition, step, body } => {
            ForLowerer::new(init, condition, step, body).lower(builder)
        }
        Statement::Break => BreakLowerer.lower(builder),
        Statement::Continue => ContinueLowerer.lower(builder),
        Statement::Expression(expr) => {
            builder.lower_expression_statement(expr)
        }
        Statement::Block(statements) => {
            builder.lower_block_statement(statements)
        }
        Statement::Const(_) => {
            // Constants are handled during semantic analysis
            Ok(())
        }
    }
}
```

**File**: `crates/compiler/mir/src/lowering/stmt/let_stmt.rs`

```rust
use crate::{MirBuilder, Value, Pattern};
use cairo_m_compiler_parser::parser::{Expression, Spanned};

pub struct LetStatementLowerer<'a> {
    pattern: &'a Pattern,
    value: &'a Spanned<Expression>,
}

impl<'a> LetStatementLowerer<'a> {
    pub fn new(pattern: &'a Pattern, value: &'a Spanned<Expression>) -> Self {
        Self { pattern, value }
    }
}

impl<'a> StatementLowerer for LetStatementLowerer<'a> {
    fn lower(&mut self, builder: &mut MirBuilder) -> Result<(), String> {
        // Move lines 58-167 from lowering/stmt.rs here
        // Simplified version after optimization extraction:

        let rhs_value = builder.lower_expression(self.value)?;
        let scope_id = builder.get_expression_scope(self.value)?;

        // Handle aggregate literals that return addresses directly
        if let Pattern::Identifier(name) = self.pattern
            && let Expression::StructLiteral { .. } | Expression::Tuple(_) = self.value.value()
        {
            if let Value::Operand(addr) = rhs_value {
                builder.bind_identifier(name, addr, scope_id)?;
                return Ok(());
            }
        }

        builder.lower_pattern(self.pattern, rhs_value, scope_id)?;
        Ok(())
    }
}
```

**Similar structure for other statements** - Move each `lower_X_statement` to
its own file.

### 3.2 Migration Checklist

- [ ] Move `lower_let_statement` (lines 58-167) → `let_stmt.rs`
- [ ] Move `lower_return_statement` (lines 169-210) → `flow.rs`
- [ ] Move `lower_assignment_statement` (lines 212-348) → `assign.rs`
- [ ] Move `lower_if_statement` (lines 390-455) → `control.rs`
- [ ] Move `lower_while_statement` (lines 456-510) → `loops.rs`
- [ ] Move `lower_loop_statement` (lines 511-559) → `loops.rs`
- [ ] Move `lower_for_statement` (lines 560-635) → `loops.rs`
- [ ] Move `lower_break_statement` (lines 636-650) → `flow.rs`
- [ ] Move `lower_continue_statement` (lines 651-665) → `flow.rs`

---

## Phase 4: Binary Op Refactor (Week 3, Day 1)

### 4.1 Create Type-Driven Binary Op Conversion

**File**: `crates/compiler/mir/src/instruction.rs`

**Add to BinaryOp impl**:

```rust
impl BinaryOp {
    /// Convert from parser op based on operand type
    pub fn from_parser(op: ParserBinaryOp, operand_type: &TypeData) -> Result<Self, String> {
        use ParserBinaryOp as P;
        use TypeData as T;

        let mir_op = match (op, operand_type) {
            // Felt operations
            (P::Add, T::Felt) => BinaryOp::Add,
            (P::Sub, T::Felt) => BinaryOp::Sub,
            (P::Mul, T::Felt) => BinaryOp::Mul,
            (P::Div, T::Felt) => BinaryOp::Div,
            (P::Eq, T::Felt) => BinaryOp::Eq,
            (P::Neq, T::Felt) => BinaryOp::Neq,
            (P::Less, T::Felt) => BinaryOp::Less,
            (P::Greater, T::Felt) => BinaryOp::Greater,
            (P::LessEqual, T::Felt) => BinaryOp::LessEqual,
            (P::GreaterEqual, T::Felt) => BinaryOp::GreaterEqual,

            // U32 operations
            (P::Add, T::U32) => BinaryOp::U32Add,
            (P::Sub, T::U32) => BinaryOp::U32Sub,
            (P::Mul, T::U32) => BinaryOp::U32Mul,
            (P::Div, T::U32) => BinaryOp::U32Div,
            (P::Eq, T::U32) => BinaryOp::U32Eq,
            (P::Neq, T::U32) => BinaryOp::U32Neq,
            (P::Less, T::U32) => BinaryOp::U32Less,
            (P::Greater, T::U32) => BinaryOp::U32Greater,
            (P::LessEqual, T::U32) => BinaryOp::U32LessEqual,
            (P::GreaterEqual, T::U32) => BinaryOp::U32GreaterEqual,

            // Bool operations
            (P::Eq, T::Bool) => BinaryOp::Eq,
            (P::Neq, T::Bool) => BinaryOp::Neq,
            (P::And, T::Bool) => BinaryOp::And,
            (P::Or, T::Bool) => BinaryOp::Or,

            _ => return Err(format!("Unsupported binary op {:?} for type {:?}", op, operand_type)),
        };

        Ok(mir_op)
    }

    /// Get the result type of this operation
    pub fn result_type(&self) -> MirType {
        match self {
            // Arithmetic ops return same type
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => MirType::Felt,
            BinaryOp::U32Add | BinaryOp::U32Sub | BinaryOp::U32Mul | BinaryOp::U32Div => MirType::U32,

            // Comparison ops return bool
            BinaryOp::Eq | BinaryOp::Neq | BinaryOp::Less | BinaryOp::Greater |
            BinaryOp::LessEqual | BinaryOp::GreaterEqual => MirType::Bool,

            BinaryOp::U32Eq | BinaryOp::U32Neq | BinaryOp::U32Less | BinaryOp::U32Greater |
            BinaryOp::U32LessEqual | BinaryOp::U32GreaterEqual => MirType::Bool,

            // Logical ops
            BinaryOp::And | BinaryOp::Or => MirType::Bool,
        }
    }
}
```

**Delete old function**: Remove `convert_binary_op` from `lowering/builder.rs`
(lines 127-199)

**Update call sites**:

- `lowering/expr.rs:303`: `BinaryOp::from_parser(op, &operand_type)?`
- `lowering/stmt.rs:121`: `BinaryOp::from_parser(*op, &operand_type)?`
- `lowering/stmt.rs:319`: `BinaryOp::from_parser(*op, &operand_type)?`

---

## Phase 5: Expression Visitor Pattern (Week 3, Days 2-3)

### 5.1 Create Expression Module Structure

**Create directory**: `crates/compiler/mir/src/lowering/expr/`

**File**: `crates/compiler/mir/src/lowering/expr/mod.rs`

```rust
mod binary;
mod call;
mod literal;
mod aggregate;
mod access;
mod unary;

pub use binary::BinaryOpLowerer;
pub use call::FunctionCallLowerer;
pub use literal::LiteralLowerer;
pub use aggregate::{StructLiteralLowerer, TupleLowerer, ArrayLowerer};
pub use access::{FieldAccessLowerer, IndexAccessLowerer};
pub use unary::UnaryOpLowerer;

use crate::{MirBuilder, Value};
use cairo_m_compiler_parser::parser::{Expression, Spanned};

/// Trait for lowering expression types
pub trait ExpressionLowerer {
    fn lower(&mut self, builder: &mut MirBuilder) -> Result<Value, String>;
}

/// Main dispatch for expression lowering
pub fn lower_expression(
    builder: &mut MirBuilder,
    expr: &Spanned<Expression>
) -> Result<Value, String> {
    match expr.value() {
        Expression::Literal(lit) => {
            LiteralLowerer::new(lit).lower(builder)
        }
        Expression::Identifier(name) => {
            builder.lower_identifier(name, expr.span())
        }
        Expression::BinaryOp { op, left, right } => {
            BinaryOpLowerer::new(*op, left, right, expr.span()).lower(builder)
        }
        Expression::UnaryOp { op, operand } => {
            UnaryOpLowerer::new(*op, operand).lower(builder)
        }
        Expression::FunctionCall { callee, args } => {
            FunctionCallLowerer::new(callee, args, expr.span()).lower(builder)
        }
        Expression::StructLiteral { name, fields } => {
            StructLiteralLowerer::new(name, fields, expr.span()).lower(builder)
        }
        Expression::Tuple(elements) => {
            TupleLowerer::new(elements, expr.span()).lower(builder)
        }
        Expression::Array(elements) => {
            ArrayLowerer::new(elements).lower(builder)
        }
        Expression::FieldAccess { base, field } => {
            FieldAccessLowerer::new(base, field).lower(builder)
        }
        Expression::IndexAccess { base, index } => {
            IndexAccessLowerer::new(base, index).lower(builder)
        }
    }
}
```

**File**: `crates/compiler/mir/src/lowering/expr/binary.rs`

```rust
use crate::{MirBuilder, Value, BinaryOp};
use cairo_m_compiler_parser::parser::{Expression, Spanned, BinaryOp as ParserOp};

pub struct BinaryOpLowerer<'a> {
    op: ParserOp,
    left: &'a Spanned<Expression>,
    right: &'a Spanned<Expression>,
    span: Span,
}

impl<'a> BinaryOpLowerer<'a> {
    pub fn new(
        op: ParserOp,
        left: &'a Spanned<Expression>,
        right: &'a Spanned<Expression>,
        span: Span
    ) -> Self {
        Self { op, left, right, span }
    }
}

impl<'a> ExpressionLowerer for BinaryOpLowerer<'a> {
    fn lower(&mut self, builder: &mut MirBuilder) -> Result<Value, String> {
        // Move lines 280-306 from lowering/expr.rs here

        // Lower operands
        let lhs_value = builder.lower_expression(self.left)?;
        let rhs_value = builder.lower_expression(self.right)?;

        // Get operand type for op selection
        let expr_id = builder.ctx.semantic_index
            .expression_id_by_span(self.span)
            .ok_or("No expression ID for binary op")?;
        let operand_type = builder.ctx.get_expr_type(expr_id);

        // Convert to typed MIR operation
        let mir_op = BinaryOp::from_parser(self.op, &operand_type)?;
        let result_type = mir_op.result_type();

        // Generate instruction
        let dest = builder.instr().binary(mir_op, lhs_value, rhs_value, result_type);
        Ok(Value::Operand(dest))
    }
}
```

### 5.2 Migration Checklist

- [ ] Move `lower_binary_op_expr` (lines 280-306) → `binary.rs`
- [ ] Move `lower_function_call_expr` (lines 308-383) → `call.rs`
- [ ] Move `lower_struct_literal` (lines 385-456) → `aggregate.rs`
- [ ] Move `lower_tuple_expr` (lines 458-490) → `aggregate.rs`
- [ ] Move `lower_array_expr` (lines 492-520) → `aggregate.rs`
- [ ] Move `lower_field_access` (lines 522-550) → `access.rs`
- [ ] Move `lower_index_access` (lines 552-580) → `access.rs`

---

## Phase 6: Pattern Lowering (Week 3, Day 4)

### 6.1 Create PatternLowerer

**File**: `crates/compiler/mir/src/lowering/pattern.rs`

```rust
use crate::{MirBuilder, Value, Place, Pattern};
use rustc_hash::FxHashMap;

pub struct PatternLowerer<'a, 'b> {
    pattern: &'a Pattern,
    builder: &'b mut MirBuilder,
}

impl<'a, 'b> PatternLowerer<'a, 'b> {
    pub fn new(pattern: &'a Pattern, builder: &'b mut MirBuilder) -> Self {
        Self { pattern, builder }
    }

    /// Bind a value to this pattern, returning all established bindings
    pub fn bind(&mut self, value: Value, scope_id: FileScopeId) -> Result<Vec<(Place, Value)>, String> {
        let mut bindings = Vec::new();
        self.bind_recursive(self.pattern, value, scope_id, &mut bindings)?;
        Ok(bindings)
    }

    fn bind_recursive(
        &mut self,
        pattern: &Pattern,
        value: Value,
        scope_id: FileScopeId,
        bindings: &mut Vec<(Place, Value)>
    ) -> Result<(), String> {
        match pattern {
            Pattern::Identifier(name) => {
                // Move lines 846-865 from lowering/stmt.rs
                let place = self.builder.allocate_variable(name, scope_id)?;
                bindings.push((place, value));
                Ok(())
            }
            Pattern::Tuple(patterns) => {
                // Move tuple destructuring logic from lines 774-844
                match value {
                    Value::Tuple(values) if values.len() == patterns.len() => {
                        for (pat, val) in patterns.iter().zip(values.iter()) {
                            self.bind_recursive(pat, *val, scope_id, bindings)?;
                        }
                        Ok(())
                    }
                    Value::Operand(tuple_addr) => {
                        // Load from tuple address
                        let tuple_type = self.builder.get_value_type(tuple_addr)?;
                        if let MirType::Tuple(elem_types) = tuple_type {
                            for (i, (pat, elem_type)) in patterns.iter().zip(elem_types.iter()).enumerate() {
                                let elem_val = self.builder.load_tuple_element(tuple_addr, i, elem_type)?;
                                self.bind_recursive(pat, elem_val, scope_id, bindings)?;
                            }
                            Ok(())
                        } else {
                            Err("Expected tuple type in pattern binding".to_string())
                        }
                    }
                    _ => Err("Type mismatch in tuple pattern".to_string())
                }
            }
            Pattern::Struct { name, fields } => {
                // Similar to tuple but with named fields
                self.bind_struct_pattern(name, fields, value, scope_id, bindings)
            }
            Pattern::Wildcard => {
                // No binding needed
                Ok(())
            }
        }
    }
}
```

**Remove from `lowering/stmt.rs`**:

- Lines 774-844: `lower_pattern` function
- Lines 846-865: `bind_variable` helper

---

## Phase 7: Performance & Cleanup (Week 3, Day 5 - Week 4, Day 1)

### 7.1 Add Expression Type Caching

Already implemented in Phase 1.1 with `LoweringContext::get_expr_type`

**Verify all call sites updated**:

```bash
rg "expression_semantic_type" crates/compiler/mir/src/lowering/
# Should only appear in LoweringContext::get_expr_type
```

### 7.2 Batch Semantic Queries

**Add to `LoweringContext`**:

```rust
impl<'a, 'db> LoweringContext<'a, 'db> {
    /// Preload all definitions in a scope for faster lookup
    pub fn preload_scope(&self, scope_id: FileScopeId) {
        let mut cache = self.definition_cache.borrow_mut();
        if !cache.contains_key(&scope_id) {
            let definitions = self.ctx.semantic_index.scope_definitions(scope_id);
            cache.insert(scope_id, definitions);
        }
    }

    /// Fast cached lookup for definition
    pub fn resolve_name(&self, name: &str, scope_id: FileScopeId) -> Option<DefinitionId> {
        self.preload_scope(scope_id);
        let cache = self.definition_cache.borrow();
        cache.get(&scope_id)?
            .iter()
            .find(|(_, def)| def.name() == name)
            .map(|(id, _)| *id)
    }
}
```

### 7.3 Remove Panics

**Replace all panics with proper errors**:

```bash
rg "panic!" crates/compiler/mir/src/lowering/
# Replace each with Result<_, String> error
```

Example replacement:

```rust
// Before
panic!("MIR: Unsupported binary op {:?} with type {:?}", op, operand_type);

// After
return Err(format!("MIR: Unsupported binary op {:?} with type {:?}", op, operand_type));
```

---

## Testing Strategy

### Continuous Testing Protocol

**After EVERY file change**:

```bash
# Quick smoke test
cargo test -p cairo-m-compiler-mir test_simple_function

# Full test suite
cargo test -p cairo-m-compiler-mir

# Check snapshots
cargo insta review

# Run full compiler
cargo run -- -i crates/compiler/mir/tests/test_data/functions/simple.cm
```

### Regression Test Suite

**Create file**: `crates/compiler/mir/tests/refactoring_regression.rs`

```rust
use std::fs;
use std::path::Path;

#[test]
fn test_mir_generation_unchanged() {
    let test_files = glob::glob("tests/test_data/**/*.cm").unwrap();

    for entry in test_files {
        let path = entry.unwrap();
        let content = fs::read_to_string(&path).unwrap();

        // Generate MIR
        let db = TestDatabase::new();
        let file = db.new_file("test.cm", content);
        let mir = lower_to_mir(&db, file);

        // Compare with snapshot
        let snapshot_name = path.file_stem().unwrap().to_str().unwrap();
        insta::assert_snapshot!(snapshot_name, format!("{:#?}", mir));
    }
}

#[test]
fn test_optimizations_still_applied() {
    // Test that tuple destructuring still works
    let code = r#"
        fn test() {
            let (a, b) = (1, 2);
            let (x, y) = get_tuple();
        }
    "#;

    let mir = compile_to_mir(code);

    // Should not see intermediate tuple allocation
    assert!(!mir.contains("tuple_alloc"));

    // Should see direct assignments
    assert!(mir.contains("a = 1"));
    assert!(mir.contains("b = 2"));
}

#[test]
fn test_binary_op_fusion() {
    let code = r#"
        fn test() {
            let x = a + b;
        }
    "#;

    let mir = compile_to_mir(code);

    // Should see single fused instruction
    assert_eq!(mir.instructions.len(), 1);
    assert!(matches!(mir.instructions[0], Instruction::BinaryOp { .. }));
}
```

### Performance Benchmarking

**Create file**: `crates/compiler/mir/benches/lowering_benchmark.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_mir_lowering(c: &mut Criterion) {
    let test_files = [
        ("simple", include_str!("../tests/test_data/simple.cm")),
        ("complex", include_str!("../tests/test_data/complex.cm")),
        ("large", include_str!("../tests/test_data/large.cm")),
    ];

    for (name, content) in &test_files {
        c.bench_function(&format!("mir_lowering_{}", name), |b| {
            b.iter(|| {
                let db = TestDatabase::new();
                let file = db.new_file("test.cm", black_box(*content));
                lower_to_mir(&db, file)
            });
        });
    }
}

criterion_group!(benches, benchmark_mir_lowering);
criterion_main!(benches);
```

Run benchmarks before and after:

```bash
cargo bench --bench lowering_benchmark -- --save-baseline before
# ... do refactoring ...
cargo bench --bench lowering_benchmark -- --baseline before
```

---

## Rollback Procedures

### Phase Rollback

Each phase is designed to be independently revertible:

1. **Git branch per phase**:

```bash
git checkout -b refactor/mir-phase-0
# ... implement phase 0 ...
git commit -m "refactor(mir): Phase 0 - Add Place/Value types and InstrBuilder"

git checkout -b refactor/mir-phase-1
# ... implement phase 1 ...
```

2. **Feature flags for gradual rollout**:

```rust
#[cfg(feature = "new_mir_lowering")]
mod new_lowering;

#[cfg(not(feature = "new_mir_lowering"))]
mod old_lowering;
```

3. **Parallel implementations**:

```rust
pub fn lower_to_mir(db: &dyn SemanticDb, file: File) -> MirModule {
    if std::env::var("USE_NEW_MIR").is_ok() {
        new_lowering::lower_to_mir(db, file)
    } else {
        old_lowering::lower_to_mir(db, file)
    }
}
```

### Emergency Revert

If critical issues found:

```bash
# Revert to last known good commit
git revert HEAD~n..HEAD

# Or switch implementations
export USE_OLD_MIR=1
cargo build --release
```

---

## Success Metrics

### Code Quality Metrics

| Metric                 | Current                     | Target | Measurement             |
| ---------------------- | --------------------------- | ------ | ----------------------- |
| Largest function (LoC) | 270 (`lower_let_statement`) | < 50   | `tokei --files`         |
| MirBuilder size (LoC)  | 800+                        | < 200  | `wc -l`                 |
| Cyclomatic complexity  | > 20                        | < 10   | `cargo clippy`          |
| Test coverage          | 70%                         | > 90%  | `cargo tarpaulin`       |
| Compilation time       | Baseline                    | -15%   | `cargo build --timings` |

### Functional Metrics

- [ ] All existing tests pass
- [ ] No performance regression in benchmarks
- [ ] Snapshot tests unchanged (semantically)
- [ ] New language features easier to add

### Development Velocity Metrics

- [ ] Time to add new statement type: < 2 hours
- [ ] Time to add new expression type: < 1 hour
- [ ] Time to add new optimization: < 30 minutes
- [ ] Code review time reduced by 50%

---

## Timeline Summary

| Week | Phase | Tasks                          | Risk   |
| ---- | ----- | ------------------------------ | ------ |
| 1    | 0-1   | Foundation, State separation   | Low    |
| 1-2  | 2     | Extract optimizations          | Low    |
| 2    | 3     | Statement modularization       | Medium |
| 3    | 4-5   | Binary ops, Expression visitor | Low    |
| 3-4  | 6-7   | Patterns, Performance          | Low    |
| 4    | -     | Testing, documentation, review | Low    |

**Total estimated time**: 4 weeks for single developer, 2-3 weeks with pair

---

## Appendix: Feature Preservation Checklist

### Critical Features That Must Work

- [x] **Tuple destructuring**: `let (a, b) = (1, 2)`
- [x] **Function tuple unpacking**: `let (x, y) = func()`
- [x] **Binary op optimization**: `let x = a + b` (single instruction)
- [x] **Dead code elimination**: Unused variables don't allocate
- [x] **Aggregate handling**: Structs/tuples return addresses
- [x] **Loop contexts**: Break/continue with nested loops
- [x] **Type-specific ops**: U32 vs Felt operations
- [x] **Void function calls**: Function calls as statements
- [x] **Block scoping**: Correct scope resolution
- [x] **Return values**: Single and tuple returns

### Edge Cases to Test

1. **Nested patterns**: `let ((a, b), c) = ((1, 2), 3)`
2. **Shadowing**: `let x = 1; { let x = 2; }`
3. **Early returns**: Return inside loops/conditions
4. **Break/continue**: In nested loops
5. **Mixed types**: U32 and Felt in same function
6. **Empty blocks**: `{}`
7. **Unreachable code**: After return/break
8. **Side effects**: Function calls for side effects only

---

## Phase 1 Completion Assessment

### Phase 1 Implementation Summary

Phase 1 has been successfully completed with the following achievements:

1. **LoweringContext/MirState Split (Phase 1.1)**: ✅
   - Successfully separated immutable context from mutable state
   - Created `LoweringContext` with database, semantic info, and caches
   - Created `MirState` with function-specific mutable data
   - Added expression type caching for performance

2. **CfgBuilder Creation (Phase 1.2)**: ✅
   - Implemented comprehensive CFG builder with clean API
   - Resolved borrowing issues using `CfgState` return type
   - Provides specialized methods for control flow patterns

### Legacy Code Assessment

After thorough analysis of the codebase, several areas of legacy code remain
that need attention:

#### 1. **Direct Block Creation** ❌

Multiple locations still directly call
`self.state.mir_function.add_basic_block()` instead of using CfgBuilder:

- `stmt.rs:405, 412, 440` - If statement handling
- `stmt.rs:530-532` - While loop block creation
- `stmt.rs:573-574` - Loop statement blocks
- `stmt.rs:615-617` - For loop blocks

**Recommendation**: These should use `self.new_block()` or CfgBuilder methods.

#### 2. **Direct Block ID Assignment** ❌

Manual assignment of `self.state.current_block_id` occurs in:

- `stmt.rs:422, 431, 451, 461, 490` - If statement flow
- `stmt.rs:541, 546, 558` - While loop flow
- `stmt.rs:584, 596` - Loop statement flow
- `stmt.rs:627, 632, 639, 647` - For loop flow

**Recommendation**: These should use `self.switch_to_block()` for proper state
management.

#### 3. **Direct Terminator Setting** ❌

Two instances of direct terminator manipulation:

- `stmt.rs:467, 486` - Setting jump terminators on merge blocks

**Recommendation**: Should use CfgBuilder's `set_block_terminator` method.

#### 4. **InstrBuilder Not Used** ❌

The InstrBuilder was created but is never instantiated or used. All instruction
creation still uses:

- Direct `Instruction::binary_op()` calls
- Direct `Instruction::unary_op()` calls
- Direct `Instruction::load/store/call()` calls
- Manual `self.add_instruction()` calls

**Recommendation**: Add `instr_builder()` method to MirBuilder and refactor
instruction creation.

#### 5. **Unused CfgBuilder Methods** ⚠️

Several useful CfgBuilder methods are not being utilized:

- `create_if_blocks()` - Could simplify if statement handling
- `create_loop_blocks()` - Could simplify loop creation
- `create_for_loop_blocks()` - Could simplify for loop creation
- `jump_to()` - Could simplify control flow
- `create_and_switch_to_block()` - Could simplify block transitions

### Technical Debt Introduced

1. **Unused field warning**: `definition_cache` in LoweringContext is never used
2. **Backup files**: Multiple `.bak` files exist in the lowering directory

### Recommendations Before Phase 2

Before proceeding to Phase 2, we should:

1. **Complete Builder Integration** (Priority: HIGH)
   - Refactor all direct block creation to use CfgBuilder
   - Refactor all direct state mutations to use proper methods
   - Implement and use InstrBuilder throughout
   - Remove direct terminator manipulations

2. **Cleanup** (Priority: MEDIUM)
   - Remove backup files (`.bak`, `.bak2`, `.bak3`)
   - Remove or implement `definition_cache`
   - Use CfgBuilder's convenience methods where appropriate

3. **Documentation** (Priority: LOW)
   - Add usage examples for builders
   - Document the new architecture

### Conclusion

While Phase 1's architectural changes are complete and functional, the
integration is incomplete. The new builders exist but aren't fully utilized
throughout the codebase. This creates a inconsistent codebase where some
operations use the new clean APIs while others use legacy direct manipulation.

**Recommendation**: Add a "Phase 1.3: Complete Builder Integration" before
moving to Phase 2.

---

## Conclusion

This refactoring plan provides a safe, incremental path to transform the MIR
lowering module from a monolithic, hard-to-maintain codebase into a modular,
extensible architecture. Each phase is designed to be independently valuable
while building toward the complete vision.

The key to success is maintaining comprehensive testing throughout and being
willing to iterate on the design as real-world usage patterns emerge.
