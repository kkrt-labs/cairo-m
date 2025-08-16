# Task 004: Critical Assignment SSA Rebinding

## Priority: CRITICAL

## Dependencies: Task 003 (Variable-SSA pass)

## Why

Assignment statements in the current MIR implementation rely heavily on memory
operations (`store`, `load`, `get_element_ptr`) even for simple value
assignments. This memory-centric approach creates several issues:

1. **Unnecessary Memory Traffic**: Simple assignments like `x = 5` or `x = y`
   generate memory allocation, store operations, and subsequent loads, even when
   these could be handled as pure SSA value bindings.

2. **Complex Aggregate Mutations**: Field assignments like `point.x = 10`
   currently require loading the entire struct, modifying it in memory, and
   storing it back. This creates inefficient memory access patterns and
   complicates optimization.

3. **SSA Form Violations**: The current assignment lowering bypasses SSA form by
   using memory as a shared mutable state, requiring heavy optimization passes
   (SROA, Mem2Reg) to restore proper SSA semantics.

4. **Optimization Barriers**: Memory-based assignments prevent many
   optimizations that could work on pure SSA values, such as constant folding,
   dead code elimination, and copy propagation.

The new assignment implementation needs to work seamlessly with SSA rebinding,
where variable assignments create new SSA versions rather than memory mutations,
and field assignments use `InsertField` operations to create modified aggregate
values.

## What

Refactor the `lower_assignment_statement` function in `stmt.rs` to implement
SSA-based assignment semantics that eliminate unnecessary memory operations for
value types. The changes include:

### Core Assignment Types

1. **Identifier Assignment** (`x = value`):
   - For variables bound to SSA values: Create new SSA version and rebind the
     variable
   - For variables bound to addresses: Keep existing memory-based behavior (for
     arrays, explicit address-taking)
   - Integration with Variable-SSA pass for proper phi node insertion at control
     flow merges

2. **Member Access Assignment** (`struct.field = value`):
   - Use `InsertField` instruction to create new struct value with modified
     field
   - Rebind the base variable to the new struct value
   - Handle nested field access (`obj.inner.field`) with chained `InsertField`
     operations

3. **Tuple Index Assignment** (`tuple.0 = value`):
   - Use `InsertTuple` instruction (similar to `InsertField` but for tuple
     indices)
   - Rebind the base variable to the new tuple value

### New Instructions Required

- `InsertField { dest, struct_val, field_name, new_value }`: Creates new struct
  with one field replaced
- `InsertTuple { dest, tuple_val, index, new_value }`: Creates new tuple with
  one element replaced

### Integration Points

- **Variable-SSA Pass**: Mark assignment sites as definition points for phi node
  insertion
- **Type System**: Preserve type information through value replacements
- **Error Handling**: Maintain comprehensive diagnostics for assignment
  validation

## How

### Phase 1: Add New Aggregate Instructions

1. **Extend `InstructionKind` enum** in `mir/src/instruction.rs`:

   ```rust
   /// Insert a new field value into a struct: `dest = insert_field(struct_val, "field", value)`
   InsertField {
       dest: ValueId,
       struct_val: Value,
       field_name: String,
       new_value: Value,
       struct_ty: MirType,
   },

   /// Insert a new element value into a tuple: `dest = insert_tuple(tuple_val, index, value)`
   InsertTuple {
       dest: ValueId,
       tuple_val: Value,
       index: usize,
       new_value: Value,
       tuple_ty: MirType,
   },
   ```

2. **Implement instruction builders** in `mir/src/builder/instr_builder.rs`:

   ```rust
   pub fn insert_field(
       &mut self,
       struct_val: Value,
       field_name: String,
       new_value: Value,
       struct_ty: MirType,
   ) -> ValueId {
       let dest = self.mir_function.new_typed_value_id(struct_ty.clone());
       self.add_instruction(Instruction::insert_field(dest, struct_val, field_name, new_value, struct_ty));
       dest
   }

   pub fn insert_tuple(
       &mut self,
       tuple_val: Value,
       index: usize,
       new_value: Value,
       tuple_ty: MirType,
   ) -> ValueId {
       let dest = self.mir_function.new_typed_value_id(tuple_ty.clone());
       self.add_instruction(Instruction::insert_tuple(dest, tuple_val, index, new_value, tuple_ty));
       dest
   }
   ```

3. **Update validation and pretty printing** to support new instructions.

### Phase 2: Refactor Assignment Statement Lowering

1. **Modify `lower_assignment_statement`** in `mir/src/lowering/stmt.rs`:

   ```rust
   pub(super) fn lower_assignment_statement(
       &mut self,
       lhs: &Spanned<Expression>,
       rhs: &Spanned<Expression>,
   ) -> Result<(), String> {
       match lhs.value() {
           Expression::Identifier(name) => {
               // Case 1: Simple identifier assignment (x = value)
               self.lower_identifier_assignment(name, lhs.span(), rhs)
           }
           Expression::MemberAccess { base, field } => {
               // Case 2: Field assignment (obj.field = value)
               self.lower_field_assignment(base, field, rhs)
           }
           Expression::TupleIndex { base, index } => {
               // Case 3: Tuple element assignment (tuple.0 = value)
               self.lower_tuple_assignment(base, *index, rhs)
           }
           _ => {
               // Fallback to existing memory-based implementation for unsupported LHS types
               self.lower_assignment_statement_legacy(lhs, rhs)
           }
       }
   }
   ```

2. **Implement identifier assignment logic**:

   ```rust
   fn lower_identifier_assignment(
       &mut self,
       name: &Spanned<String>,
       lhs_span: SimpleSpan,
       rhs: &Spanned<Expression>,
   ) -> Result<(), String> {
       // Get scope and resolve variable
       let scope_id = self.get_expression_scope(lhs_span)?;
       let (def_idx, _) = self.resolve_variable_definition(name, scope_id)?;
       let def_id = DefinitionId::new(self.ctx.db, self.ctx.file, def_idx);
       let mir_def_id = self.convert_definition_id(def_id);

       // Check if variable is currently bound to an SSA value or address
       if let Some(current_value_id) = self.state.definition_to_value.get(&mir_def_id) {
           if let Some(value_type) = self.state.mir_function.get_value_type(*current_value_id) {
               if !matches!(value_type, MirType::Pointer(_)) {
                   // Variable is bound to SSA value - create new binding
                   let rhs_value = self.lower_expression(rhs)?;
                   self.bind_variable_value(name, scope_id, rhs_value)?;

                   // Mark as assignment site for Variable-SSA pass
                   self.mark_variable_assignment(mir_def_id, self.state.current_block_id);
                   return Ok(());
               }
           }
       }

       // Fallback to memory-based assignment
       self.lower_assignment_statement_legacy(lhs, rhs)
   }
   ```

3. **Implement field assignment logic**:

   ```rust
   fn lower_field_assignment(
       &mut self,
       base: &Spanned<Expression>,
       field: &Spanned<String>,
       rhs: &Spanned<Expression>,
   ) -> Result<(), String> {
       // Must be an identifier base for SSA rebinding
       let Expression::Identifier(base_name) = base.value() else {
           return self.lower_assignment_statement_legacy(lhs, rhs);
       };

       let scope_id = self.get_expression_scope(base.span())?;
       let base_def_id = self.resolve_and_convert_definition(base_name, scope_id)?;

       // Check if base variable is bound to SSA value
       if let Some(current_value_id) = self.state.definition_to_value.get(&base_def_id) {
           if let Some(struct_type) = self.state.mir_function.get_value_type(*current_value_id) {
               if let MirType::Struct { .. } = struct_type && !matches!(struct_type, MirType::Pointer(_)) {
                   // Perform SSA field insertion
                   let rhs_value = self.lower_expression(rhs)?;
                   let current_struct = Value::operand(*current_value_id);

                   let new_struct_id = self.instr().insert_field(
                       current_struct,
                       field.value().clone(),
                       rhs_value,
                       struct_type.clone(),
                   );

                   // Rebind the base variable to the new struct
                   self.state.definition_to_value.insert(base_def_id, new_struct_id);
                   self.mark_variable_assignment(base_def_id, self.state.current_block_id);
                   return Ok(());
               }
           }
       }

       // Fallback to memory-based assignment
       self.lower_assignment_statement_legacy(lhs, rhs)
   }
   ```

### Phase 3: Variable Binding Helpers

1. **Add variable assignment tracking**:

   ```rust
   /// Mark a variable as assigned in the current block for Variable-SSA pass
   fn mark_variable_assignment(&mut self, def_id: MirDefinitionId, block_id: BasicBlockId) {
       self.state.variable_assignments
           .entry(def_id)
           .or_insert_with(Vec::new)
           .push(block_id);
   }

   /// Bind a variable to a new SSA value (replacement binding)
   fn bind_variable_value(
       &mut self,
       name: &Spanned<String>,
       scope: FileScopeId,
       value: Value,
   ) -> Result<(), String> {
       let (def_idx, _) = self.resolve_variable_definition(name, scope)?;
       let def_id = DefinitionId::new(self.ctx.db, self.ctx.file, def_idx);
       let mir_def_id = self.convert_definition_id(def_id);

       match value {
           Value::Operand(value_id) => {
               self.state.definition_to_value.insert(mir_def_id, value_id);
           }
           Value::Literal(_) => {
               // Create SSA value for literal
               let var_type = self.get_variable_type(def_id)?;
               let value_id = self.state.mir_function.new_typed_value_id(var_type.clone());
               self.instr().assign(value_id, value, var_type);
               self.state.definition_to_value.insert(mir_def_id, value_id);
           }
           _ => return Err("Unsupported value type for variable binding".to_string()),
       }
       Ok(())
   }
   ```

### Phase 4: Integration with Variable-SSA Pass

1. **Pass assignment information to Variable-SSA**:
   - Store assignment sites in `MirBuilder` state
   - Export assignment information for phi node placement
   - Ensure Variable-SSA pass runs before validation

2. **Update builder state structure**:

   ```rust
   pub struct BuilderState {
       // ... existing fields ...

       /// Tracks which variables are assigned in which blocks
       /// Used by Variable-SSA pass for phi node placement
       variable_assignments: HashMap<MirDefinitionId, Vec<BasicBlockId>>,
   }
   ```

### Phase 5: Testing Strategy

1. **Unit Tests** for new instructions:

   ```rust
   #[test]
   fn test_insert_field_instruction() {
       // Test InsertField instruction creation and validation
   }

   #[test]
   fn test_insert_tuple_instruction() {
       // Test InsertTuple instruction creation and validation
   }
   ```

2. **Integration Tests** for assignment lowering:

   ```rust
   #[test]
   fn test_simple_assignment_ssa() {
       // Test: x = 5; should create SSA binding, not memory ops
   }

   #[test]
   fn test_field_assignment_ssa() {
       // Test: point.x = 10; should use InsertField, not memory ops
   }

   #[test]
   fn test_assignment_across_control_flow() {
       // Test: if (cond) { x = 1; } else { x = 2; } use x;
       // Should require phi nodes from Variable-SSA pass
   }
   ```

3. **Snapshot Tests** for MIR output:
   - Verify that simple assignments generate no memory operations
   - Verify that field assignments use `InsertField` instructions
   - Verify that control flow merges have appropriate phi nodes

### Phase 6: Performance Validation

1. **Benchmark Impact**:
   - Measure MIR generation time before/after changes
   - Measure optimization pass execution time reduction
   - Verify no regression in final execution performance

2. **MIR Quality Metrics**:
   - Count reduction in memory operations for aggregate-heavy code
   - Verify elimination of unnecessary `frame_alloc` instructions
   - Measure improvement in optimization effectiveness

## Implementation Notes

- **Backward Compatibility**: Keep existing memory-based assignment as fallback
  for unsupported cases (arrays, explicit address-taking)
- **Type Safety**: Ensure all `InsertField`/`InsertTuple` operations preserve
  type correctness
- **Error Recovery**: Maintain comprehensive error messages for assignment
  validation failures
- **Incremental Rollout**: Can be feature-flagged initially to allow gradual
  migration

## Success Criteria

1. Simple identifier assignments (`x = value`) generate no memory operations
   when variables are SSA-bound
2. Field assignments (`obj.field = value`) use `InsertField` instructions
   instead of memory operations
3. Assignment statements integrate correctly with Variable-SSA pass for control
   flow merges
4. All existing tests pass with new assignment implementation
5. Demonstrable reduction in memory operations for aggregate-heavy test cases
6. No performance regression in MIR generation or final execution time
