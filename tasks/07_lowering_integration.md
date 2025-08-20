# Task 7: Integrate SSA Builder with Lowering

## Goal

Replace the global `definition_to_value` mapping with SSA builder integration in
the MIR lowering process.

## Files to Modify

- `mir/src/lowering/builder.rs` - Primary changes to MirBuilder
- `mir/src/lowering/utils.rs` - Update `bind_variable` function
- `mir/src/lowering/expr.rs` - Update identifier lowering
- `mir/src/lowering/control_flow.rs` - Add block sealing
- `mir/src/lowering/stmt.rs` - Update variable binding

## Current State

- `MirState` has global
  `definition_to_value: FxHashMap<MirDefinitionId, ValueId>`
- `bind_variable()` function updates this global map
- Variable reads directly query this global map

## Required Changes

### 1. Update MirState Structure (`mir/src/lowering/builder.rs`)

Replace the global mapping with SSA builder:

```rust
use crate::ssa::SSABuilder;

/// Mutable state for the function being built
pub struct MirState<'db> {
    /// The MIR function being constructed
    pub(super) mir_function: MirFunction,
    /// The current basic block being populated with instructions
    pub(super) current_block_id: BasicBlockId,
    /// REMOVED: definition_to_value global map
    /// The DefinitionId of the function being lowered (for type information)
    pub(super) function_def_id: Option<DefinitionId<'db>>,
    /// Becomes true when a terminator like `return` is encountered.
    pub(super) is_terminated: bool,
    /// Stack of loop contexts for break/continue handling
    pub(super) loop_stack: Vec<(BasicBlockId, BasicBlockId)>,
}

/// Add SSA builder to MirBuilder
pub struct MirBuilder<'a, 'db> {
    /// Immutable compilation context
    pub(super) ctx: LoweringContext<'a, 'db>,
    /// Mutable function state
    pub(super) state: MirState<'db>,
    /// NEW: SSA builder for variable tracking
    pub(super) ssa: SSABuilder<'static>, // Lifetime to be fixed
}
```

**Lifetime Fix**: The SSA builder needs access to the MirFunction, so we need to
restructure:

```rust
pub struct MirBuilder<'a, 'db> {
    /// Immutable compilation context
    pub(super) ctx: LoweringContext<'a, 'db>,
    /// Mutable function state
    pub(super) state: MirState<'db>,
}

impl<'a, 'db> MirBuilder<'a, 'db> {
    /// Create and use SSA builder temporarily
    fn with_ssa<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut SSABuilder, &mut MirState<'db>, &LoweringContext<'a, 'db>) -> R,
    {
        let mut ssa = SSABuilder::new(&mut self.state.mir_function);
        f(&mut ssa, &mut self.state, &self.ctx)
    }
}
```

### 2. Rewrite and add SSA Variable Binding Methods

Add new methods to `MirBuilder`:

```rust
impl<'a, 'db> MirBuilder<'a, 'db> {
    /// REWRITE: bind_variable now binds a variable to a value using SSA tracking
    pub fn bind_variable(
        &mut self,
        ident_name: &str,
        ident_span: SimpleSpan,
        value: Value,
    ) -> Result<(), String> {
        self.with_ssa(|ssa, state, ctx| {
            // Get semantic information for the identifier
            let expr_id = ctx.semantic_index
                .expression_id_by_span(ident_span)
                .ok_or_else(|| format!("No ExpressionId for identifier {}", ident_name))?;

            let expr_info = ctx.semantic_index
                .expression(expr_id)
                .ok_or_else(|| format!("No ExpressionInfo for identifier {}", ident_name))?;

            // Resolve to semantic definition
            let (def_idx, definition) = ctx.semantic_index
                .resolve_name_to_definition(ident_name, expr_info.scope_id)
                .ok_or_else(|| format!("Failed to resolve identifier {}", ident_name))?;

            let def_id = DefinitionId::new(ctx.db, ctx.file, def_idx);
            let mir_def_id = MirDefinitionId {
                definition_index: def_id.id_in_file(ctx.db).index(),
                file_id: ctx.file_id,
            };

            // Get variable type for proper handling
            let var_type = definition_semantic_type(ctx.db, ctx.crate_id, def_id);
            let mir_type = MirType::from_semantic_type(ctx.db, var_type);

            // Convert value to ValueId if needed
            let value_id = match value {
                Value::Operand(id) => id,
                Value::Literal(_) => {
                    // Create assignment instruction for literals
                    let temp_id = state.mir_function.new_typed_value_id(mir_type);
                    let assign_instr = Instruction::assign(temp_id, value, mir_type);

                    if let Some(block) = state.mir_function.basic_blocks.get_mut(state.current_block_id) {
                        block.push_instruction(assign_instr);
                    }
                    temp_id
                }
                Value::Error => {
                    // Create error placeholder
                    state.mir_function.new_typed_value_id(mir_type)
                }
            };

            // Bind using SSA
            ssa.write_variable(mir_def_id, state.current_block_id, value_id);
            Ok(())
        })
    }

    /// Read a variable using SSA tracking
    pub fn read_variable(
        &mut self,
        ident_name: &str,
        ident_span: SimpleSpan,
    ) -> Result<ValueId, String> {
        self.with_ssa(|ssa, state, ctx| {
            // Get semantic information
            let expr_id = ctx.semantic_index
                .expression_id_by_span(ident_span)
                .ok_or_else(|| format!("No ExpressionId for identifier {}", ident_name))?;

            let expr_info = ctx.semantic_index
                .expression(expr_id)
                .ok_or_else(|| format!("No ExpressionInfo for identifier {}", ident_name))?;

            // Resolve to definition
            let (def_idx, _definition) = ctx.semantic_index
                .resolve_name_to_definition(ident_name, expr_info.scope_id)
                .ok_or_else(|| format!("Failed to resolve identifier {}", ident_name))?;

            let def_id = DefinitionId::new(ctx.db, ctx.file, def_idx);
            let mir_def_id = MirDefinitionId {
                definition_index: def_id.id_in_file(ctx.db).index(),
                file_id: ctx.file_id,
            };

            // Read using SSA
            let value_id = ssa.read_variable(mir_def_id, state.current_block_id);
            Ok(value_id)
        })
    }

    /// Seal a block - no more predecessors will be added
    /// This must be called when the predecessor set of a block is finalized
    pub fn seal_block(&mut self, block_id: BasicBlockId) {
        // Mark in CFG builder first
        let mut cfg = self.cfg();
        cfg.seal_block(block_id);

        // Then complete incomplete phis in SSA builder
        self.with_ssa(|ssa, _state, _ctx| {
            ssa.seal_block(block_id);
        });
    }

    /// Mark a block as filled - all local statements processed
    pub fn mark_block_filled(&mut self, block_id: BasicBlockId) {
        let mut cfg = self.cfg();
        cfg.mark_block_filled(block_id);

        self.with_ssa(|ssa, _state, _ctx| {
            ssa.mark_block_filled(block_id);
        });
    }
}
```

### 4. Update Identifier Lowering (`mir/src/lowering/expr.rs`)

Replace variable lookup in `lower_identifier`:

```rust
// In lower_identifier function, replace:
// OLD: Direct lookup in definition_to_value map

// NEW: SSA-based variable reading
fn lower_identifier(
    builder: &mut MirBuilder<'a, 'db>,
    ident: &Spanned<Identifier>,
) -> Result<Value, String> {
    let ident_name = ident.value();

    // Try SSA variable lookup first
    match builder.read_variable_ssa(ident_name, ident.span()) {
        Ok(value_id) => {
            // Check if this is an array pointer that needs special handling
            let value_type = builder.state.mir_function.get_value_type(value_id);
            if let Some(MirType::Pointer(pointee)) = value_type {
                if matches!(**pointee, MirType::Array { .. }) {
                    // Array variable - return the pointer as-is
                    return Ok(Value::Operand(value_id));
                }
            }

            // Regular variable - return the value
            Ok(Value::Operand(value_id))
        }
        Err(err) => {
            // Variable not found in SSA - this should create a diagnostic
            // For now, return an error value
            eprintln!("Variable lookup failed: {}", err);
            Ok(Value::Error)
        }
    }
}
```

### 5. Add Block Sealing to Control Flow (`mir/src/lowering/control_flow.rs`)

Add sealing at appropriate points in control flow lowering.

**Key insight from Braun et al. section 2.3:** A block should be sealed when no
further predecessors will be added to it. For control flow constructs:

- **If statements**: Seal `then` and `else` blocks after branching to them
  (their predecessors are known)
- **Loops**: Seal loop body entry after entry edge, but NOT loop header until
  backedge is added
- **Merge blocks**: Seal after ALL incoming branches are connected

Example patterns:

```rust
// In lower_if_statement:
pub fn lower_if_statement(
    builder: &mut MirBuilder<'a, 'db>,
    if_stmt: &IfStatement,
) -> Result<(), String> {
    let (then_block, else_block, merge_block) = builder.create_if_blocks();

    // Lower condition in current block
    let condition_value = lower_expression(builder, &if_stmt.condition)?;

    // Branch to then/else blocks
    builder.terminate_with_branch(condition_value, then_block, else_block);

    // CRITICAL: Seal then and else blocks - no more predecessors will be added
    builder.seal_block(then_block);
    builder.seal_block(else_block);

    // Lower then branch
    builder.switch_to_block(then_block);
    lower_statement(builder, &if_stmt.then_branch)?;
    builder.terminate_with_jump(merge_block);

    // Lower else branch
    builder.switch_to_block(else_block);
    if let Some(else_branch) = &if_stmt.else_branch {
        lower_statement(builder, else_branch)?;
    }
    builder.terminate_with_jump(merge_block);

    // NOW: Seal merge block - all predecessors (then, else) have been connected
    builder.seal_block(merge_block);
    builder.switch_to_block(merge_block);

    Ok(())
}

// For while loops (showing the tricky case):
pub fn lower_while_statement(
    builder: &mut MirBuilder<'a, 'db>,
    while_stmt: &WhileStatement,
) -> Result<(), String> {
    let (header_block, body_block, exit_block) = builder.create_loop_blocks();

    // Jump to header
    builder.terminate_with_jump(header_block);

    // DO NOT seal header yet - backedge will be added later
    builder.switch_to_block(header_block);

    // Lower condition and branch
    let condition = lower_expression(builder, &while_stmt.condition)?;
    builder.terminate_with_branch(condition, body_block, exit_block);

    // Seal body_block (only one predecessor: header)
    builder.seal_block(body_block);
    builder.switch_to_block(body_block);

    // Lower body
    lower_statement(builder, &while_stmt.body)?;

    // Add backedge to header
    builder.terminate_with_jump(header_block);

    // NOW seal header - all predecessors (entry, backedge) are connected
    builder.seal_block(header_block);

    // Seal exit block
    builder.seal_block(exit_block);
    builder.switch_to_block(exit_block);

    Ok(())
}
```

### 6. Update Constructor

Update `MirBuilder::new()` to initialize without global map:

```rust
impl<'a, 'db> MirBuilder<'a, 'db> {
    pub fn new(
        db: &'db dyn SemanticDb,
        file: File,
        semantic_index: &'a SemanticIndex,
        function_mapping: &'a FxHashMap<DefinitionId<'db>, (&'a Definition, FunctionId)>,
        file_id: u64,
        crate_id: Crate,
    ) -> Self {
        let mir_function = MirFunction::new(String::new());
        let entry_block = mir_function.entry_block;

        // ... existing context setup ...

        let state = MirState {
            mir_function,
            current_block_id: entry_block,
            // REMOVED: definition_to_value: FxHashMap::default(),
            function_def_id: None,
            is_terminated: false,
            loop_stack: Vec::new(),
        };

        Self { ctx, state }
    }
}
```

## Legacy Code to Remove

AFTER this task completes:

1. **In `mir/src/lowering/builder.rs`**:
   - Remove `definition_to_value: FxHashMap<MirDefinitionId, ValueId>` field
     from `MirState`
   - Remove initialization of this field in `MirBuilder::new()`

2. **In `mir/src/lowering/utils.rs`**:
   - Remove the entire old `bind_variable` function implementation
   - Remove any direct access to `state.definition_to_value`

3. **In `mir/src/lowering/expr.rs`**:
   - Remove direct lookups in `state.definition_to_value`
   - Remove `builder.state.definition_to_value.get(&mir_def_id)` calls

4. **Any other files with**:
   - Direct access to `definition_to_value` field
   - Manual variable binding that bypasses the SSA system

## Migration Strategy

1. **Phase 1**: Add SSA methods alongside existing methods
2. **Phase 2**: Update call sites to use new SSA methods
3. **Phase 3**: Remove old methods and global mapping
4. **Phase 4**: Verify all tests pass

## Testing

- Ensure all existing lowering tests pass
- Test that variables are properly tracked across blocks
- Test phi node creation at merge points
- Test sealing prevents incorrect phi completion
- Test that error recovery still works

## Success Criteria

- ✅ Global `definition_to_value` mapping completely removed
- ✅ All variable binding goes through SSA builder
- ✅ Block sealing happens at correct points in control flow
- ✅ Phi nodes are created at merge points automatically
- ✅ All existing MIR functionality continues to work
- ✅ All tests pass
