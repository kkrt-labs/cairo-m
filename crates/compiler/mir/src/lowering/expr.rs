//! # Expression Lowering
//!
//! This module contains the trait and implementations for lowering expressions
//! from the AST to MIR values.

use cairo_m_compiler_parser::parser::{BinaryOp, Expression, Spanned, UnaryOp};
use cairo_m_compiler_semantic::builtins::{BuiltinFn, is_builtin_function_name};
use cairo_m_compiler_semantic::definition::DefinitionKind;
use cairo_m_compiler_semantic::place::FileScopeId;
use cairo_m_compiler_semantic::semantic_index::{DefinitionId, ExpressionId};
use cairo_m_compiler_semantic::type_resolution::expression_semantic_type;
use cairo_m_compiler_semantic::types::TypeData;

use super::builder::{CallResult, MirBuilder};
use crate::instruction::CalleeSignature;
use crate::{Instruction, MirType, Place, Value};

/// Trait for lowering expressions to MIR values
pub trait LowerExpr<'a> {
    fn lower_expression(&mut self, expr: &Spanned<Expression>) -> Result<LoweredExpr, String>;
}

#[derive(Clone, Debug)]
pub struct LoweredExpr {
    value: Value,
    place: Option<Place>,
}

impl LoweredExpr {
    pub const fn new(value: Value) -> Self {
        Self { value, place: None }
    }

    pub const fn with_place(value: Value, place: Place) -> Self {
        Self {
            value,
            place: Some(place),
        }
    }

    pub const fn value(&self) -> &Value {
        &self.value
    }

    pub fn into_value(self) -> Value {
        self.value
    }

    pub const fn place(&self) -> Option<&Place> {
        self.place.as_ref()
    }

    pub fn into_place(self) -> Option<Place> {
        self.place
    }
}

impl From<Value> for LoweredExpr {
    fn from(value: Value) -> Self {
        Self::new(value)
    }
}

impl<'a, 'db> LowerExpr<'a> for MirBuilder<'a, 'db> {
    fn lower_expression(&mut self, expr: &Spanned<Expression>) -> Result<LoweredExpr, String> {
        // First, get the ExpressionId and its associated info
        let expr_id = self.expr_id(expr.span())?;

        let expr_info = self
            .ctx
            .semantic_index
            .expression(expr_id)
            .ok_or_else(|| format!("MIR: No ExpressionInfo for ID {expr_id:?}"))?;

        let current_scope_id = expr_info.scope_id;

        // Use expr_info.ast_node instead of expr.value()
        match &expr_info.ast_node {
            Expression::Literal(n, _) => Ok(LoweredExpr::new(Value::integer(*n as u32))),
            Expression::BooleanLiteral(b) => Ok(LoweredExpr::new(Value::boolean(*b))),
            Expression::New { elem_type, count } => {
                // Compute cells = count * elem_slots, where elem_slots depends on T
                let sem_elem_type = cairo_m_compiler_semantic::type_resolution::resolve_ast_type(
                    self.ctx.db,
                    self.ctx.crate_id,
                    self.ctx.file,
                    elem_type.clone(),
                    expr_info.scope_id,
                );
                let elem_mir_ty = MirType::from_semantic_type(self.ctx.db, sem_elem_type);
                let elem_slots = crate::DataLayout::value_size_of(&elem_mir_ty);

                // Lower count expression to a value
                let count_val = self.lower_expression(count)?;
                // Enforce: count must be felt (single-slot). Upstream validation should catch
                // this, but MIR lowering defends here to avoid width mismatches.
                let count_sem_ty = expression_semantic_type(
                    self.ctx.db,
                    self.ctx.crate_id,
                    self.ctx.file,
                    self.expr_id(count.span())?,
                    None,
                );
                let count_mir_ty = MirType::from_semantic_type(self.ctx.db, count_sem_ty);
                if !matches!(count_mir_ty, MirType::Felt) {
                    return Err("MIR: new count must be felt".to_string());
                }

                // If element occupies more than one slot, multiply.
                // Ensure we produce a felt (single-slot) cells value for HeapAllocCells.
                let cells_val = if elem_slots == 1 {
                    count_val
                } else {
                    // Literal scale factor
                    let lit = crate::Value::integer(elem_slots as u32);
                    // Multiply in felt domain: cells = count * elem_slots
                    let dest_tmp = self.state.mir_function.new_typed_value_id(MirType::Felt);
                    self.instr().binary_op_to(
                        crate::BinaryOp::Mul,
                        dest_tmp,
                        count_val.into_value(),
                        lit,
                    );
                    LoweredExpr::new(Value::operand(dest_tmp))
                };

                // Destination pointer value with element type information
                let dest = self
                    .state
                    .mir_function
                    .new_typed_value_id(MirType::pointer(elem_mir_ty));
                // Emit heap allocation instruction
                self.instr()
                    .add_instruction(Instruction::heap_alloc_cells(dest, cells_val.into_value()));
                Ok(LoweredExpr::new(Value::operand(dest)))
            }
            Expression::Identifier(name) => self.lower_identifier(name, current_scope_id),
            Expression::UnaryOp { op, expr } => self.lower_unary_op(*op, expr, expr_id),
            Expression::BinaryOp { op, left, right } => {
                self.lower_binary_op(*op, left, right, expr_id)
            }
            Expression::Parenthesized(inner) => self.lower_expression(inner),
            Expression::FunctionCall { callee, args } => {
                self.lower_function_call_expr(callee, args, expr_id)
            }
            Expression::MemberAccess { object, field } => {
                self.lower_member_access(object, field, expr_id)
            }
            Expression::IndexAccess { array, index } => {
                self.lower_array_index(array, index, expr_id)
            }
            Expression::StructLiteral { name: _, fields } => {
                self.lower_struct_literal(fields, expr_id)
            }
            Expression::Tuple(elements) => self.lower_tuple_literal(elements, expr_id),
            Expression::TupleIndex { tuple, index } => self.lower_tuple_index(tuple, *index),
            Expression::ArrayLiteral(elements) => self.lower_array_literal(elements, expr_id),
            Expression::ArrayRepeat { element, count } => {
                self.lower_array_repeat(element, *count.value() as usize, expr_id)
            }
            Expression::Cast {
                expr,
                target_type: _,
            } => self.lower_cast(expr, expr_id),
        }
    }
}

// Individual expression lowering methods
impl<'a, 'db> MirBuilder<'a, 'db> {
    /// Resolves an identifier by looking up its definition in the semantic index.
    ///
    /// With the value-first approach, most variables are bound to values directly.
    /// Only arrays use pointers and require loading.
    fn lower_identifier(
        &mut self,
        name: &Spanned<String>,
        _scope_id: FileScopeId,
    ) -> Result<LoweredExpr, String> {
        // Use the builder-recorded mapping from this identifier expression to its definition
        let expr_id = self.expr_id(name.span())?;
        if let Some((def_idx, def)) = self
            .ctx
            .semantic_index
            .definition_for_identifier_expr(expr_id)
        {
            let def_id = DefinitionId::new(self.ctx.db, self.ctx.file, def_idx);

            // Check if this is a constant definition
            if let DefinitionKind::Const(const_ref) = &def.kind {
                // Constants need to be evaluated to their values
                if let Some(value_expr_id) = const_ref.value_expr_id {
                    // Get the constant's value expression from the semantic index
                    if let Some(expr_info) = self.ctx.semantic_index.expression(value_expr_id) {
                        // Lower the constant's value expression
                        let const_expr =
                            Spanned::new(expr_info.ast_node.clone(), expr_info.ast_span);
                        // Lower under const context to mark aggregates as read-only
                        let prev = self.state.in_const_context;
                        self.state.in_const_context = true;
                        let value = self.lower_expression(&const_expr);
                        self.state.in_const_context = prev;
                        return value;
                    }
                }
                return Err(format!(
                    "Constant '{}' has no value expression",
                    name.value()
                ));
            }

            let _mir_def_id = self.convert_definition_id(def_id);

            // Look up the MIR value for this definition (for variables, not constants)
            if let Ok(var_value) = self.read_variable(name.value(), name.span()) {
                // It's a value (primitive, struct, tuple) - use directly
                return Ok(LoweredExpr::new(Value::operand(var_value)));
            } else {
                panic!("Unexpected error: could not read variable {}", name.value());
            }
        }

        // If we can't resolve the identifier, return an error value for recovery
        Ok(LoweredExpr::new(Value::error()))
    }

    fn lower_unary_op(
        &mut self,
        op: UnaryOp,
        expr: &Spanned<Expression>,
        expr_id: ExpressionId,
    ) -> Result<LoweredExpr, String> {
        let expr_value = self.lower_expression(expr)?.into_value();

        // Query semantic type system for result type based on this expression
        let result_type = self.ctx.get_expr_type(expr_id);

        // Use the new unary_op API that allocates its own destination
        let dest = self.instr().unary_op(op, expr_value, result_type);

        // Register unary op result as a Value

        Ok(LoweredExpr::new(Value::operand(dest)))
    }

    fn lower_binary_op(
        &mut self,
        op: BinaryOp,
        left: &Spanned<Expression>,
        right: &Spanned<Expression>,
        expr_id: ExpressionId,
    ) -> Result<LoweredExpr, String> {
        let lhs_value = self.lower_expression(left)?.into_value();
        let rhs_value = self.lower_expression(right)?.into_value();

        // Query semantic type system for result type based on this expression
        let result_type = self.ctx.get_expr_type(expr_id);
        let dest = self.state.mir_function.new_typed_value_id(result_type);

        // Register binary op result as a Value

        // Get the type of the left operand to determine the correct binary operation
        let left_expr_id = self.expr_id(left.span())?;

        let left_type = expression_semantic_type(
            self.ctx.db,
            self.ctx.crate_id,
            self.ctx.file,
            left_expr_id,
            None,
        );
        let left_type_data = left_type.data(self.ctx.db);

        let typed_op = crate::BinaryOp::from_parser(op, &left_type_data)?;
        self.instr()
            .binary_op_to(typed_op, dest, lhs_value, rhs_value);
        Ok(LoweredExpr::new(Value::operand(dest)))
    }

    fn lower_function_call_expr(
        &mut self,
        callee: &Spanned<Expression>,
        args: &[Spanned<Expression>],
        expr_id: ExpressionId,
    ) -> Result<LoweredExpr, String> {
        // Handle built-in assert(...) in expression position as well.
        // Emit the same MIR as in statement position, then return unit.
        if let Expression::Identifier(name) = callee.value() {
            if is_builtin_function_name(name.value()) == Some(BuiltinFn::Assert) {
                // Retrieve the call span fom the current expression id
                let call_span = self
                    .ctx
                    .semantic_index
                    .expression(expr_id)
                    .map(|info| info.ast_span)
                    .ok_or_else(|| format!("MIR: No ExpressionInfo for call ID {expr_id:?}"))?;

                self.lower_assert_call(args, call_span)?;
                return Ok(LoweredExpr::new(Value::unit()));
            }
        }

        match self.lower_function_call(callee, args, expr_id)? {
            CallResult::Single(value) => Ok(LoweredExpr::new(value)),
            CallResult::Tuple(values) => {
                // For expression context, we need to return a single value
                // Use MakeTuple to create a value-based tuple from the returned values
                let tuple_type = self.ctx.get_expr_type(expr_id);

                // Create a tuple value using MakeTuple instruction
                let tuple_value = self.make_tuple(values, tuple_type);
                Ok(LoweredExpr::new(Value::operand(tuple_value)))
            }
        }
    }

    fn lower_member_access(
        &mut self,
        object: &Spanned<Expression>,
        field: &Spanned<String>,
        expr_id: ExpressionId,
    ) -> Result<LoweredExpr, String> {
        // NOTE: When arrays are implemented, they should use memory-based access:
        // - Arrays should use get_element_ptr + load for element access
        // - Arrays should NOT use ExtractTupleElement or similar value-based operations
        // - Use MirType::requires_memory_path() to check
        // Lower the struct expression to get a value and/or place
        let lowered_object = self.lower_expression(object)?;

        // Query semantic type system for the field type
        let field_type = self.ctx.get_expr_type(expr_id);

        // If the object has a place (e.g., arr[i]), extend it with a field projection
        // and emit a Load directly from memory.
        if let Some(p) = lowered_object.place() {
            let mut place = p.clone();
            place = place.with_field(field.value().clone());

            let dest_id = self
                .state
                .mir_function
                .new_typed_value_id(field_type.clone());
            self.instr()
                .add_instruction(Instruction::load(dest_id, place.clone(), field_type));
            Ok(LoweredExpr::with_place(Value::operand(dest_id), place))
        } else {
            // Pure value path: extract by value
            let struct_val = lowered_object.into_value();
            let field_dest =
                self.extract_struct_field(struct_val, field.value().clone(), field_type);
            Ok(LoweredExpr::new(Value::operand(field_dest)))
        }
    }

    pub(super) fn lower_function_call(
        &mut self,
        callee: &Spanned<Expression>,
        args: &[Spanned<Expression>],
        expr_id: ExpressionId,
    ) -> Result<CallResult, String> {
        // First, resolve the callee to a FunctionId
        let func_id = self.resolve_callee_expression(callee)?;

        // Lower the arguments
        let mut arg_values = Vec::new();
        for arg in args {
            arg_values.push(self.lower_expression(arg)?.into_value());
        }

        // Get the callee's signature by looking up the function definition
        let (param_types, return_types) = self.get_function_signature(func_id)?;

        // Get the return type of the function
        let semantic_type =
            expression_semantic_type(self.ctx.db, self.ctx.crate_id, self.ctx.file, expr_id, None);

        // Check if the return type is a tuple
        match semantic_type.data(self.ctx.db) {
            TypeData::Tuple(element_types) => {
                // Function returns a tuple - create multiple destination values
                let mut dests = Vec::new();
                for elem_type in element_types {
                    let mir_type = MirType::from_semantic_type(self.ctx.db, elem_type);
                    let dest = self.state.mir_function.new_typed_value_id(mir_type);
                    // Register each return value as a Value since it's computed by the function
                    dests.push(dest);
                }

                // Create the CalleeSignature
                let signature = CalleeSignature {
                    param_types,
                    return_types,
                };

                // Create the call instruction with the signature
                let call_instr = Instruction::call(dests.clone(), func_id, arg_values, signature);
                self.instr().add_instruction(call_instr);

                // Return the tuple values directly
                Ok(CallResult::Tuple(
                    dests.into_iter().map(Value::operand).collect(),
                ))
            }
            _ => {
                // Single return value
                let return_type = MirType::from_semantic_type(self.ctx.db, semantic_type);
                let dest = self.state.mir_function.new_typed_value_id(return_type);
                // Register return value as a Value since it's computed by the function

                // Create the CalleeSignature
                let signature = CalleeSignature {
                    param_types,
                    return_types,
                };

                // Create the call instruction with the signature
                let call_instr = Instruction::call(vec![dest], func_id, arg_values, signature);
                self.instr().add_instruction(call_instr);

                Ok(CallResult::Single(Value::operand(dest)))
            }
        }
    }

    fn lower_struct_literal(
        &mut self,
        fields: &[(Spanned<String>, Spanned<Expression>)],
        expr_id: ExpressionId,
    ) -> Result<LoweredExpr, String> {
        // NEW: Value-based struct creation
        // Query semantic type system for struct type
        let struct_type = self.ctx.get_expr_type(expr_id);

        // Lower each field to a value
        let mut field_values = Vec::new();
        for (field_name, field_expr) in fields {
            let field_val = self.lower_expression(field_expr)?.into_value();
            field_values.push((field_name.value().clone(), field_val));
        }

        // Create the struct using a single MakeStruct instruction
        let struct_dest = self.make_struct(field_values, struct_type);

        Ok(LoweredExpr::new(Value::operand(struct_dest)))
    }

    fn lower_tuple_literal(
        &mut self,
        elements: &[Spanned<Expression>],
        expr_id: ExpressionId,
    ) -> Result<LoweredExpr, String> {
        // NEW: Value-based tuple creation
        if elements.is_empty() {
            // Empty tuple - return proper unit value
            return Ok(LoweredExpr::new(Value::unit()));
        }

        // Lower each element to a value
        let mut element_values = Vec::new();
        for element_expr in elements {
            let element_val = self.lower_expression(element_expr)?.into_value();
            element_values.push(element_val);
        }

        // Query semantic type system for the tuple type
        let tuple_type = self.ctx.get_expr_type(expr_id);

        // Create the tuple using a single MakeTuple instruction
        let tuple_dest = self.make_tuple(element_values, tuple_type);

        Ok(LoweredExpr::new(Value::operand(tuple_dest)))
    }

    fn lower_tuple_index(
        &mut self,
        tuple: &Spanned<Expression>,
        index: usize,
    ) -> Result<LoweredExpr, String> {
        // Lower the tuple expression to get a value and/or place
        let lowered_tuple = self.lower_expression(tuple)?;

        // Determine element type from tuple type
        let tuple_mir_type = self.expr_mir_type(tuple.span())?;
        let element_mir_type = match &tuple_mir_type {
            MirType::Tuple(types) => types
                .get(index)
                .ok_or_else(|| format!("Tuple index {} out of bounds", index))?
                .clone(),
            _ => return Err("TupleIndex on non-tuple type".to_string()),
        };

        if let Some(p) = lowered_tuple.place() {
            // Extend place with tuple projection and load from memory
            let mut place = p.clone();
            place = place.with_tuple(index);
            let dest_id = self
                .state
                .mir_function
                .new_typed_value_id(element_mir_type.clone());
            self.instr().add_instruction(Instruction::load(
                dest_id,
                place.clone(),
                element_mir_type,
            ));
            Ok(LoweredExpr::with_place(Value::operand(dest_id), place))
        } else {
            // Pure value path: extract by value
            let tuple_val = lowered_tuple.into_value();
            let element_dest = self.extract_tuple_element(tuple_val, index, element_mir_type);
            Ok(LoweredExpr::new(Value::operand(element_dest)))
        }
    }

    fn lower_array_literal(
        &mut self,
        elements: &[Spanned<Expression>],
        expr_id: ExpressionId,
    ) -> Result<LoweredExpr, String> {
        // Lower each element to a value
        let mut element_values = Vec::new();
        for element_expr in elements {
            let element_val = self.lower_expression(element_expr)?.into_value();
            element_values.push(element_val);
        }

        // Query semantic type system for the array type
        let array_type = self.ctx.get_expr_type(expr_id);

        // Get element type from the array type
        let element_mir_type = match &array_type {
            MirType::FixedArray { element_type, .. } => (**element_type).clone(),
            _ => return Err("ArrayLiteral does not have array type".to_string()),
        };

        // Create the array using MakeFixedArray instruction (const context respected in builder)
        let array_dest = self.make_fixed_array(element_values, element_mir_type);

        Ok(LoweredExpr::new(Value::operand(array_dest)))
    }

    fn lower_array_repeat(
        &mut self,
        element: &Spanned<Expression>,
        count: usize,
        expr_id: ExpressionId,
    ) -> Result<LoweredExpr, String> {
        // Lower the element expression once
        let elem_value = self.lower_expression(element)?.into_value();

        // Query semantic array type to obtain element MIR type
        let array_type = self.ctx.get_expr_type(expr_id);
        let element_mir_type = match &array_type {
            MirType::FixedArray { element_type, .. } => (**element_type).clone(),
            _ => return Err("ArrayRepeat does not have array type".to_string()),
        };

        // Build element values by repetition
        let elements: Vec<Value> = std::iter::repeat_n(elem_value, count).collect();

        // Create the array using MakeFixedArray instruction (const context respected)
        let array_dest = self.make_fixed_array(elements, element_mir_type);

        Ok(LoweredExpr::new(Value::operand(array_dest)))
    }

    fn lower_cast(
        &mut self,
        expr: &Spanned<Expression>,
        expr_id: ExpressionId,
    ) -> Result<LoweredExpr, String> {
        // Lower the source expression
        let source_value = self.lower_expression(expr)?.into_value();

        // Get the source type from semantic analysis
        let source_expr_id = self.expr_id(expr.span())?;
        let source_semantic_type = expression_semantic_type(
            self.ctx.db,
            self.ctx.crate_id,
            self.ctx.file,
            source_expr_id,
            None,
        );
        let source_type = MirType::from_semantic_type(self.ctx.db, source_semantic_type);

        // Get the target type from semantic analysis
        let target_semantic_type =
            expression_semantic_type(self.ctx.db, self.ctx.crate_id, self.ctx.file, expr_id, None);
        let target_type = MirType::from_semantic_type(self.ctx.db, target_semantic_type);

        let dest_id = self
            .state
            .mir_function
            .new_typed_value_id(target_type.clone());
        let cast_instr = Instruction::cast(dest_id, source_value, source_type, target_type);
        self.instr().add_instruction(cast_instr);
        Ok(LoweredExpr::new(Value::operand(dest_id)))
    }

    fn lower_array_index(
        &mut self,
        array: &Spanned<Expression>,
        index: &Spanned<Expression>,
        _expr_id: ExpressionId,
    ) -> Result<LoweredExpr, String> {
        // Lower the array expression to get a value and/or a place
        let array_lowered = self.lower_expression(array)?;
        let array_val = *array_lowered.value();

        // Get the MIR type of the array
        let array_mir_type = self.expr_mir_type(array.span())?;

        // Get element type. Prefer MIR information (FixedArray or Pointer),
        // otherwise fall back to semantic pointer element type.
        let element_mir_type = match &array_mir_type {
            MirType::FixedArray { element_type, .. } => (**element_type).clone(),
            MirType::Pointer { element } => (**element).clone(),
            _ => {
                // Fallback to semantic type: support pointers (T*)
                let array_expr_id = self.expr_id(array.span())?;
                let sem_ty = expression_semantic_type(
                    self.ctx.db,
                    self.ctx.crate_id,
                    self.ctx.file,
                    array_expr_id,
                    None,
                );
                match sem_ty.data(self.ctx.db) {
                    cairo_m_compiler_semantic::types::TypeData::Pointer { element_type } => {
                        MirType::from_semantic_type(self.ctx.db, element_type)
                    }
                    _ => return Err("IndexAccess on non-array type".to_string()),
                }
            }
        };

        // Lower index expression and reuse it for both load and potential store
        let index_lowered = self.lower_expression(index)?;
        let index_value = *index_lowered.value();

        // Build the place for this indexed element
        // Preserve any existing place (e.g., arr[i].nested)[j] by extending it,
        // otherwise fall back to using the operand base value.
        let mut place = if let Some(p) = array_lowered.place().cloned() {
            p
        } else {
            match array_val {
                Value::Operand(id) => Place::new(id),
                _ => return Err("Array index requires operand base".to_string()),
            }
        };
        place = place.with_index(index_value);

        // Emit load for the element value
        let dest = self
            .state
            .mir_function
            .new_typed_value_id(element_mir_type.clone());
        self.instr()
            .add_instruction(Instruction::load(dest, place.clone(), element_mir_type));

        Ok(LoweredExpr::with_place(Value::operand(dest), place))
    }
}
