# 002 - Critical: Value-Based Aggregate Lowering

**Priority**: CRITICAL  
**Dependencies**: Task 001 (requires first-class aggregate instructions)

## Why

The current MIR lowering strategy treats all aggregates as memory allocations,
which creates several fundamental problems:

1. **Memory pollution**: Simple `let p = Point { x, y }` becomes `frame_alloc` +
   multiple `store` operations
2. **Optimization burden**: Requires expensive SROA and Mem2Reg passes to
   recover efficient SSA form
3. **Compilation performance**: Memory-based lowering forces complex dominance
   analysis for simple operations
4. **Code complexity**: Aggregate operations become multi-instruction sequences
   that obscure intent

By refactoring lowering to generate value-based aggregate instructions directly,
we eliminate the root cause of these issues and produce cleaner, more
optimizable MIR from the start.

## What

Transform the lowering pipeline to emit value-based aggregate instructions
instead of memory operations for:

1. **Tuple literals**: `(a, b, c)` → `MakeTuple` instruction (not
   `frame_alloc` + `store`s)
2. **Struct literals**: `Point { x, y }` → `MakeStruct` instruction
3. **Tuple indexing**: `tuple.1` → `ExtractTupleElement` (not
   `get_element_ptr` + `load`)
4. **Field access**: `point.x` → `ExtractStructField` (not `get_element_ptr` +
   `load`)
5. **Let bindings**: Direct SSA value binding for aggregate expressions
6. **Function calls**: Proper handling of multi-value returns and tuple contexts

The result will be MIR that represents aggregates as first-class values
throughout the pipeline.

## How

### Phase 1: Refactor Expression Lowering

1. **Edit `crates/compiler/mir/src/lowering/expr.rs`**:

   **Update `lower_tuple_literal()` (around line 150)**:

   ```rust
   fn lower_tuple_literal(&mut self, tuple_literal: &Expression) -> Result<Value, LoweringError> {
       let elements = match tuple_literal {
           Expression::Tuple(elements) => elements,
           _ => return Err(LoweringError::InvalidExpression),
       };

       // OLD: frame_alloc + multiple stores
       // let tuple_addr = self.frame_alloc(tuple_ty);
       // for (i, element_expr) in elements.iter().enumerate() {
       //     let element_val = self.lower_expression(element_expr)?;
       //     self.store_tuple_element(tuple_addr, i, element_val);
       // }

       // NEW: Single MakeTuple instruction
       let mut element_values = Vec::new();
       for element_expr in elements {
           let element_val = self.lower_expression(element_expr)?;
           element_values.push(element_val);
       }

       let tuple_ty = self.get_expr_type(tuple_literal)?;
       let dest = self.builder.make_tuple(element_values, tuple_ty);
       Ok(Value::Operand(dest))
   }
   ```

   **Update `lower_struct_literal()` (around line 200)**:

   ```rust
   fn lower_struct_literal(&mut self, struct_literal: &Expression) -> Result<Value, LoweringError> {
       let (struct_ty, field_exprs) = match struct_literal {
           Expression::StructLiteral { ty, fields } => (ty, fields),
           _ => return Err(LoweringError::InvalidExpression),
       };

       // OLD: frame_alloc + multiple field stores
       // let struct_addr = self.frame_alloc(struct_ty);
       // for (field_name, field_expr) in field_exprs {
       //     let field_val = self.lower_expression(field_expr)?;
       //     self.store_field(struct_addr, field_name, field_val);
       // }

       // NEW: Single MakeStruct instruction
       let mut field_values = Vec::new();
       for (field_name, field_expr) in field_exprs {
           let field_val = self.lower_expression(field_expr)?;
           field_values.push((field_name.clone(), field_val));
       }

       let mir_struct_ty = self.semantic_ty_to_mir_ty(struct_ty)?;
       let dest = self.builder.make_struct(field_values, mir_struct_ty);
       Ok(Value::Operand(dest))
   }
   ```

   **Update `lower_tuple_index()` (around line 250)**:

   ```rust
   fn lower_tuple_index(&mut self, tuple_expr: &Expression, index: usize) -> Result<Value, LoweringError> {
       // OLD: lower_lvalue_expression + get_element_ptr + load
       // let tuple_addr = self.lower_lvalue_expression(tuple_expr)?;
       // let element_addr = self.builder.get_element_ptr(tuple_addr, vec![index]);
       // let element_val = self.builder.load(element_addr, element_ty);

       // NEW: Direct ExtractTupleElement on value
       let tuple_val = self.lower_expression(tuple_expr)?;
       let element_ty = self.get_tuple_element_type(tuple_expr, index)?;
       let dest = self.builder.extract_tuple_element(tuple_val, index, element_ty);
       Ok(Value::Operand(dest))
   }
   ```

   **Update `lower_member_access()` (around line 300)**:

   ```rust
   fn lower_member_access(&mut self, struct_expr: &Expression, field_name: &str) -> Result<Value, LoweringError> {
       // OLD: lower_lvalue_expression + get_element_ptr + load
       // let struct_addr = self.lower_lvalue_expression(struct_expr)?;
       // let field_addr = self.builder.get_element_ptr_field(struct_addr, field_name);
       // let field_val = self.builder.load(field_addr, field_ty);

       // NEW: Direct ExtractStructField on value
       let struct_val = self.lower_expression(struct_expr)?;
       let field_ty = self.get_struct_field_type(struct_expr, field_name)?;
       let dest = self.builder.extract_struct_field(struct_val, field_name.to_string(), field_ty);
       Ok(Value::Operand(dest))
   }
   ```

### Phase 2: Update Statement Lowering

2. **Edit `crates/compiler/mir/src/lowering/stmt.rs`**:

   **Update `lower_let_statement()` (around line 100)**:

   ```rust
   fn lower_let_statement(&mut self, let_stmt: &Statement) -> Result<(), LoweringError> {
       let (pattern, rhs_expr) = match let_stmt {
           Statement::Let { pattern, expression } => (pattern, expression),
           _ => return Err(LoweringError::InvalidStatement),
       };

       match pattern {
           Pattern::Identifier(var_name) => {
               // NEW: Direct SSA value binding for aggregates
               let rhs_value = self.lower_expression(rhs_expr)?;
               self.bind_variable(var_name, rhs_value)?;
               Ok(())
           }
           Pattern::Tuple(element_patterns) => {
               // IMPROVED: Simplified tuple destructuring
               let tuple_val = self.lower_expression(rhs_expr)?;
               for (i, element_pattern) in element_patterns.iter().enumerate() {
                   if let Pattern::Identifier(var_name) = element_pattern {
                       let element_ty = self.get_tuple_element_type(rhs_expr, i)?;
                       let element_val = self.builder.extract_tuple_element(tuple_val.clone(), i, element_ty);
                       self.bind_variable(var_name, Value::Operand(element_val))?;
                   }
               }
               Ok(())
           }
           // Other patterns unchanged
           _ => self.lower_let_statement_legacy(let_stmt),
       }
   }
   ```

### Phase 3: Function Call Handling

3. **Update `lower_function_call_expr()` in `expr.rs` (around line 400)**:

   ```rust
   fn lower_function_call_expr(&mut self, call_expr: &Expression) -> Result<Value, LoweringError> {
       let (func_id, args) = self.extract_call_info(call_expr)?;

       // Lower arguments
       let mut arg_values = Vec::new();
       for arg_expr in args {
           let arg_val = self.lower_expression(arg_expr)?;
           arg_values.push(arg_val);
       }

       // Generate call instruction
       let call_result_values = self.builder.function_call(func_id, arg_values)?;

       // NEW: Handle tuple context for multi-value returns
       let expected_ty = self.get_expr_type(call_expr)?;
       match (call_result_values.len(), &expected_ty) {
           (1, _) => Ok(call_result_values[0].clone()),
           (n, MirType::Tuple(_)) if n > 1 => {
               // Multi-value return in tuple context: synthesize tuple
               let tuple_dest = self.builder.make_tuple(call_result_values, expected_ty);
               Ok(Value::Operand(tuple_dest))
           }
           (n, _) if n > 1 => {
               // Multi-value return, non-tuple context: return first value
               Ok(call_result_values[0].clone())
           }
           _ => Err(LoweringError::CallReturnMismatch),
       }
   }
   ```

### Phase 4: Return Statement Handling

4. **Update `lower_return_statement()` in `stmt.rs` (around line 200)**:

   ```rust
   fn lower_return_statement(&mut self, return_stmt: &Statement) -> Result<(), LoweringError> {
       let return_expr = match return_stmt {
           Statement::Return(Some(expr)) => expr,
           Statement::Return(None) => {
               self.builder.return_void();
               return Ok(());
           }
           _ => return Err(LoweringError::InvalidStatement),
       };

       let return_val = self.lower_expression(return_expr)?;

       // NEW: Handle tuple returns by extracting elements
       let function_return_types = self.current_function_return_types();
       match (function_return_types.len(), &return_val) {
           (1, _) => {
               self.builder.return_value(vec![return_val]);
           }
           (n, Value::Operand(tuple_val_id)) if n > 1 => {
               // Extract tuple elements for multi-value return
               let mut return_values = Vec::new();
               for i in 0..n {
                   let element_ty = function_return_types[i].clone();
                   let element_val = self.builder.extract_tuple_element(
                       Value::Operand(*tuple_val_id), i, element_ty
                   );
                   return_values.push(Value::Operand(element_val));
               }
               self.builder.return_value(return_values);
           }
           _ => return Err(LoweringError::ReturnTypeMismatch),
       }
       Ok(())
   }
   ```

### Phase 5: Backward Compatibility

5. **Add fallback handling** for complex cases that still need memory:

   ```rust
   fn should_use_memory_lowering(&self, expr: &Expression) -> bool {
       match expr {
           // Arrays always use memory for now
           Expression::ArrayLiteral(_) => true,
           // Explicit address-of operations
           Expression::AddressOf(_) => true,
           // Large aggregates (configurable threshold)
           Expression::StructLiteral { .. } if self.is_large_struct(expr) => true,
           _ => false,
       }
   }

   fn lower_expression(&mut self, expr: &Expression) -> Result<Value, LoweringError> {
       if self.should_use_memory_lowering(expr) {
           self.lower_expression_legacy_memory(expr)
       } else {
           self.lower_expression_value_based(expr)
       }
   }
   ```

### Phase 6: Helper Methods

6. **Add type helper methods**:

   ```rust
   fn get_tuple_element_type(&self, tuple_expr: &Expression, index: usize) -> Result<MirType, LoweringError> {
       let tuple_ty = self.get_expr_type(tuple_expr)?;
       match tuple_ty {
           MirType::Tuple(element_types) => {
               element_types.get(index)
                   .cloned()
                   .ok_or(LoweringError::TupleIndexOutOfBounds)
           }
           _ => Err(LoweringError::ExpectedTupleType),
       }
   }

   fn get_struct_field_type(&self, struct_expr: &Expression, field_name: &str) -> Result<MirType, LoweringError> {
       let struct_ty = self.get_expr_type(struct_expr)?;
       // Implementation depends on semantic type system integration
       self.semantic_db.get_struct_field_type(&struct_ty, field_name)
   }
   ```

### Phase 7: Testing

7. **Create comprehensive tests in `tests/mir_generation_tests.rs`**:

   ```rust
   #[test]
   fn test_value_based_tuple_literal() {
       let source = r#"
           fn test() -> (M31, M31) {
               let t = (1, 2);
               return t;
           }
       "#;

       let mir = compile_to_mir(source).expect("compilation failed");
       let mir_text = mir.pretty_print();

       // Should contain MakeTuple, not frame_alloc
       assert!(mir_text.contains("maketuple"));
       assert!(!mir_text.contains("framealloc"));
       assert!(!mir_text.contains("store"));
   }

   #[test]
   fn test_value_based_struct_literal() {
       let source = r#"
           struct Point { x: M31, y: M31 }
           fn test() -> Point {
               let p = Point { x: 10, y: 20 };
               return p;
           }
       "#;

       let mir = compile_to_mir(source).expect("compilation failed");
       let mir_text = mir.pretty_print();

       // Should contain MakeStruct, not memory operations
       assert!(mir_text.contains("makestruct"));
       assert!(!mir_text.contains("framealloc"));
   }

   #[test]
   fn test_value_based_field_access() {
       let source = r#"
           struct Point { x: M31, y: M31 }
           fn test() -> M31 {
               let p = Point { x: 42, y: 24 };
               return p.x;
           }
       "#;

       let mir = compile_to_mir(source).expect("compilation failed");
       let mir_text = mir.pretty_print();

       // Should use ExtractStructField, not GEP + load
       assert!(mir_text.contains("extractfield"));
       assert!(!mir_text.contains("get_element_ptr"));
       assert!(!mir_text.contains("load"));
   }
   ```

### Phase 8: Incremental Migration

8. **Add feature flag support** (prepare for Task 012):

   ```rust
   pub struct LoweringContext {
       use_value_based_aggregates: bool,
       // other fields...
   }

   impl LoweringContext {
       pub fn new_with_config(config: &PipelineConfig) -> Self {
           Self {
               use_value_based_aggregates: config.aggregate_mir_enabled(),
               // ...
           }
       }
   }
   ```

## Definition of Done

- [x] Tuple literals generate `MakeTuple` instructions instead of memory
      operations
- [x] Struct literals generate `MakeStruct` instructions instead of memory
      operations
- [x] Tuple indexing uses `ExtractTupleElement` instead of `get_element_ptr` +
      `load`
- [x] Field access uses `ExtractStructField` instead of `get_element_ptr` +
      `load`
- [ ] Let statements bind aggregate SSA values directly (partially done)
- [ ] Function calls properly handle multi-value returns in tuple contexts
      (partial)
- [ ] Return statements extract tuple elements for multi-value returns (partial)
- [x] Backward compatibility maintained for arrays and address-of operations
- [x] All new MIR generation tests pass
- [x] Existing functionality preserved (no regressions)
- [x] MIR pretty printing shows value-based operations, not memory operations

## Implementation Progress

**Date**: 2025-08-16

### Completed:

1. **Added builder methods for aggregate instructions**:
   - `make_tuple()`: Creates tuples from value lists
   - `extract_tuple_element()`: Extracts tuple elements by index
   - `make_struct()`: Creates structs from field-value pairs
   - `extract_struct_field()`: Extracts struct fields by name

2. **Updated expression lowering**:
   - `lower_tuple_literal()`: Now generates `MakeTuple` instead of
     `frame_alloc` + stores
   - `lower_struct_literal()`: Now generates `MakeStruct` instead of memory
     operations
   - `lower_tuple_index()`: Now generates `ExtractTupleElement` instead of GEP +
     load
   - `lower_member_access()`: Now generates `ExtractStructField` instead of
     GEP + load

3. **Created comprehensive test suite**:
   - Tests verify value-based operations are generated
   - Tests confirm no memory operations for simple aggregates
   - All 5 test cases pass

### Technical Notes:

- The refactoring successfully eliminates memory operations for simple aggregate
  creation and access
- MIR is now significantly cleaner and more optimization-friendly
- The implementation is backward compatible - arrays and other constructs still
  use memory as needed

### Partial/Future Work:

- Statement lowering needs more work for complete value-based handling
- Multi-value return handling in function calls needs refinement
- Return statement tuple extraction needs full implementation
- These can be completed in follow-up tasks or as part of Task 003/004

## Success Criteria

- Simple aggregate programs generate clean, memory-free MIR
- Compilation performance improves due to reduced instruction count
- Generated MIR is more readable and optimization-friendly
- Foundation is prepared for simplifying the optimization pipeline (Task 005)

This task represents the core transformation that enables all subsequent
optimizations and simplifications in the MIR refactoring effort.
