//! # Expression Lowering
//!
//! This module contains the trait and implementations for lowering expressions
//! from the AST to MIR values.

use cairo_m_compiler_parser::parser::{BinaryOp, Expression, Spanned, UnaryOp};
use cairo_m_compiler_semantic::place::FileScopeId;
use cairo_m_compiler_semantic::semantic_index::{DefinitionId, ExpressionId};
use cairo_m_compiler_semantic::type_resolution::{
    definition_semantic_type, expression_semantic_type,
};
use cairo_m_compiler_semantic::types::TypeData;

use crate::instruction::CalleeSignature;
use crate::layout::DataLayout;
use crate::{Instruction, Literal, MirType, Value};

use super::builder::{CallResult, MirBuilder};

/// Trait for lowering expressions to MIR values
pub trait LowerExpr<'a> {
    fn lower_expression(&mut self, expr: &Spanned<Expression>) -> Result<Value, String>;
    fn lower_lvalue_expression(&mut self, expr: &Spanned<Expression>) -> Result<Value, String>;
}

impl<'a, 'db> LowerExpr<'a> for MirBuilder<'a, 'db> {
    fn lower_expression(&mut self, expr: &Spanned<Expression>) -> Result<Value, String> {
        // First, get the ExpressionId and its associated info
        let expr_id = self
            .ctx
            .semantic_index
            .expression_id_by_span(expr.span())
            .ok_or_else(|| format!("MIR: No ExpressionId found for span {:?}", expr.span()))?;

        let expr_info = self
            .ctx
            .semantic_index
            .expression(expr_id)
            .ok_or_else(|| format!("MIR: No ExpressionInfo for ID {expr_id:?}"))?;

        let current_scope_id = expr_info.scope_id;

        // Special case: For TupleIndex on function calls, we need to use expr.value()
        // because expr_info.ast_node doesn't preserve the nested structure
        if let Expression::TupleIndex { tuple, index } = expr.value() {
            if let Expression::FunctionCall { callee, args } = tuple.value() {
                return self.lower_tuple_index_on_call(tuple, *index, callee, args);
            }
        }

        // Use expr_info.ast_node instead of expr.value()
        match &expr_info.ast_node {
            Expression::Literal(n, _) => Ok(Value::integer(*n as i32)),
            Expression::BooleanLiteral(b) => Ok(Value::boolean(*b)),
            Expression::Identifier(name) => self.lower_identifier(name, current_scope_id),
            Expression::UnaryOp { op, expr } => self.lower_unary_op(*op, expr, expr_id),
            Expression::BinaryOp { op, left, right } => {
                self.lower_binary_op(*op, left, right, expr_id)
            }
            Expression::FunctionCall { callee, args } => {
                self.lower_function_call_expr(callee, args, expr_id)
            }
            Expression::MemberAccess { object, field } => {
                self.lower_member_access(object, field, expr_id)
            }
            Expression::IndexAccess { array, index } => {
                self.lower_index_access(array, index, expr_id)
            }
            Expression::StructLiteral { name: _, fields } => {
                self.lower_struct_literal(fields, expr_id)
            }
            Expression::Tuple(elements) => self.lower_tuple_literal(elements, expr_id),
            Expression::TupleIndex { tuple, index } => self.lower_tuple_index(tuple, *index),
        }
    }

    fn lower_lvalue_expression(&mut self, expr: &Spanned<Expression>) -> Result<Value, String> {
        // First, get the ExpressionId and its associated info
        let expr_id = self
            .ctx
            .semantic_index
            .expression_id_by_span(expr.span())
            .ok_or_else(|| {
                format!(
                    "MIR: No ExpressionId found for lvalue span {:?}",
                    expr.span()
                )
            })?;
        let expr_info = self
            .ctx
            .semantic_index
            .expression(expr_id)
            .ok_or_else(|| format!("MIR: No ExpressionInfo for lvalue ID {expr_id:?}"))?;
        let current_scope_id = expr_info.scope_id;

        match &expr_info.ast_node {
            Expression::Identifier(name) => {
                // Use the correct scope_id from expr_info for resolution
                if let Some((def_idx, _)) = self
                    .ctx
                    .semantic_index
                    .resolve_name_to_definition(name.value(), current_scope_id)
                {
                    let def_id = DefinitionId::new(self.ctx.db, self.ctx.file, def_idx);
                    let mir_def_id = self.convert_definition_id(def_id);

                    // Look up the MIR value for this definition
                    if let Some(value_id) = self.state.definition_to_value.get(&mir_def_id) {
                        // For simple variables, the value ID itself represents the address
                        // In a more sophisticated system, we might need AddressOf instruction
                        return Ok(Value::operand(*value_id));
                    }
                }
                // If we can't resolve the identifier, return an error value for recovery
                Ok(Value::error())
            }
            Expression::MemberAccess { object, field } => {
                // Get the base address of the object
                let object_addr = self.lower_lvalue_expression(object)?;

                // Get the object's semantic type to calculate field offset
                let object_expr_id = self
                    .ctx
                    .semantic_index
                    .expression_id_by_span(object.span())
                    .ok_or_else(|| {
                        format!(
                            "MIR: No ExpressionId found for object span {:?}",
                            object.span()
                        )
                    })?;
                let object_mir_type = self.ctx.get_expr_type(object_expr_id);

                // Calculate the actual field offset from the type information using DataLayout
                let layout = DataLayout::new();
                let field_offset_val = layout.field_offset(&object_mir_type, field.value())
                    .ok_or_else(|| {
                        format!(
                            "Internal Compiler Error: Field '{}' not found on type '{:?}'. This indicates an issue with type information propagation.",
                            field.value(),
                            object_mir_type
                        )
                    })?;
                let field_offset = Value::integer(field_offset_val as i32);

                // Query semantic type system for field type from the member access expression
                let field_type = self.ctx.get_expr_type(expr_id);

                let dest = self.get_element_address(
                    object_addr,
                    field_offset,
                    field_type,
                    &format!("Get address of field '{}'", field.value()),
                );
                Ok(Value::operand(dest))
            }
            Expression::IndexAccess { array, index } => {
                // Get the base address of the array
                let array_addr = self.lower_lvalue_expression(array)?;

                // Lower the index expression to get the offset
                let index_value = self.lower_expression(index)?;

                // For tuples with constant indices, use the index directly since elements are consecutive
                // For general arrays/pointers, use the index directly (element size scaling would be done in a real system)
                let offset_value = index_value;

                // Query semantic type system for array element type from the index access expression
                let element_type = self.ctx.get_expr_type(expr_id);

                let dest = self.get_element_address(
                    array_addr,
                    offset_value,
                    element_type,
                    "Get address of array element",
                );
                Ok(Value::operand(dest))
            }
            Expression::TupleIndex { tuple, index } => {
                // Get the semantic type of the tuple to determine element types and offsets
                let tuple_expr_id = self
                    .ctx
                    .semantic_index
                    .expression_id_by_span(tuple.span())
                    .ok_or_else(|| "No ExpressionId for tuple in TupleIndex".to_string())?;
                // Get the MIR type of the tuple to get offset calculation
                let tuple_mir_type = self.ctx.get_expr_type(tuple_expr_id);

                // For non-function-call tuples, use the existing lvalue approach
                let tuple_addr = self.lower_lvalue_expression(tuple)?;

                // Calculate the offset for the element using DataLayout
                let layout = DataLayout::new();
                let offset = layout
                    .tuple_offset(&tuple_mir_type, *index)
                    .ok_or_else(|| format!("Invalid tuple index {} for type", index))?;

                // Get element type
                let element_mir_type = match &tuple_mir_type {
                    MirType::Tuple(types) => types
                        .get(*index)
                        .ok_or_else(|| format!("Tuple index {} out of bounds", index))?
                        .clone(),
                    _ => return Err("TupleIndex on non-tuple type".to_string()),
                };

                // Calculate element address using helper
                let element_addr = self.get_element_address(
                    tuple_addr,
                    Value::integer(offset as i32),
                    element_mir_type,
                    &format!("Get address of tuple element {} for assignment", index),
                );
                Ok(Value::operand(element_addr))
            }
            Expression::Literal(_, _)
            | Expression::BooleanLiteral(_)
            | Expression::FunctionCall { .. }
            | Expression::UnaryOp { .. }
            | Expression::BinaryOp { .. }
            | Expression::StructLiteral { .. }
            | Expression::Tuple(_) => Err(format!(
                "Expression cannot be assigned to: {:?}",
                expr_info.ast_node
            )),
        }
    }
}

// Individual expression lowering methods
impl<'a, 'db> MirBuilder<'a, 'db> {
    fn lower_identifier(
        &mut self,
        name: &Spanned<String>,
        scope_id: FileScopeId,
    ) -> Result<Value, String> {
        if let Some((def_idx, _)) = self
            .ctx
            .semantic_index
            .resolve_name_to_definition(name.value(), scope_id)
        {
            let def_id = DefinitionId::new(self.ctx.db, self.ctx.file, def_idx);
            let mir_def_id = self.convert_definition_id(def_id);

            // Look up the MIR value for this definition
            if let Some(var_value) = self.state.definition_to_value.get(&mir_def_id).copied() {
                // Check if it's a parameter (parameters are always values, not pointers)
                if self.state.mir_function.parameters.contains(&var_value) {
                    // It's a parameter - use it directly
                    return Ok(Value::operand(var_value));
                }

                // Get the type of the value to check if it's a pointer
                let value_type = self.state.mir_function.get_value_type(var_value);

                // Check if the type is a pointer - if so, we need to load
                if let Some(MirType::Pointer(_)) = value_type {
                    // It's a pointer - we need to load the value
                    let semantic_type =
                        definition_semantic_type(self.ctx.db, self.ctx.crate_id, def_id);
                    let var_type = MirType::from_semantic_type(self.ctx.db, semantic_type);
                    let loaded_value = self.state.mir_function.new_typed_value_id(var_type.clone());

                    self.instr().load_with(
                        var_type,
                        loaded_value,
                        Value::operand(var_value),
                        format!("Load variable {}", name.value()),
                    );
                    return Ok(Value::operand(loaded_value));
                } else {
                    // It's not a pointer - use it directly
                    return Ok(Value::operand(var_value));
                }
            }
        }

        // If we can't resolve the identifier, return an error value for recovery
        Ok(Value::error())
    }

    fn lower_unary_op(
        &mut self,
        op: UnaryOp,
        expr: &Spanned<Expression>,
        expr_id: ExpressionId,
    ) -> Result<Value, String> {
        let expr_value = self.lower_expression(expr)?;

        // Query semantic type system for result type based on this expression
        let result_type = self.ctx.get_expr_type(expr_id);

        // Use the new unary_op API that allocates its own destination
        let dest = self.instr().unary_op(op, expr_value, result_type);

        // Register unary op result as a Value

        Ok(Value::operand(dest))
    }

    fn lower_binary_op(
        &mut self,
        op: BinaryOp,
        left: &Spanned<Expression>,
        right: &Spanned<Expression>,
        expr_id: ExpressionId,
    ) -> Result<Value, String> {
        let lhs_value = self.lower_expression(left)?;
        let rhs_value = self.lower_expression(right)?;

        // Query semantic type system for result type based on this expression
        let result_type = self.ctx.get_expr_type(expr_id);
        let dest = self.state.mir_function.new_typed_value_id(result_type);

        // Register binary op result as a Value

        // Get the type of the left operand to determine the correct binary operation
        let left_expr_id = self
            .ctx
            .semantic_index
            .expression_id_by_span(left.span())
            .ok_or_else(|| "No expression ID for left operand".to_string())?;

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
        Ok(Value::operand(dest))
    }

    fn lower_function_call_expr(
        &mut self,
        callee: &Spanned<Expression>,
        args: &[Spanned<Expression>],
        expr_id: ExpressionId,
    ) -> Result<Value, String> {
        match self.lower_function_call(callee, args, expr_id)? {
            CallResult::Single(value) => Ok(value),
            CallResult::Tuple(values) => {
                // For expression context, we need to return a single value
                // Use MakeTuple to create a value-based tuple from the returned values
                let semantic_type = expression_semantic_type(
                    self.ctx.db,
                    self.ctx.crate_id,
                    self.ctx.file,
                    expr_id,
                    None,
                );
                let tuple_type = MirType::from_semantic_type(self.ctx.db, semantic_type);

                // Create a tuple value using MakeTuple instruction
                let tuple_value = self.make_tuple(values, tuple_type);
                Ok(Value::operand(tuple_value))
            }
        }
    }

    fn lower_member_access(
        &mut self,
        object: &Spanned<Expression>,
        field: &Spanned<String>,
        expr_id: ExpressionId,
    ) -> Result<Value, String> {
        // NOTE: When arrays are implemented, they should use memory-based access:
        // - Arrays should use get_element_ptr + load for element access
        // - Arrays should NOT use ExtractTupleElement or similar value-based operations
        // - Use array_guards::should_use_memory_lowering() to check
        // NEW: Value-based struct field extraction
        // Lower the struct expression to get a value
        let struct_val = self.lower_expression(object)?;

        // Query semantic type system for the field type
        let field_type = self.ctx.get_expr_type(expr_id);

        // Extract the field using ExtractStructField instruction
        let field_dest = self.extract_struct_field(struct_val, field.value().clone(), field_type);

        Ok(Value::operand(field_dest))
    }

    fn lower_index_access(
        &mut self,
        array: &Spanned<Expression>,
        index: &Spanned<Expression>,
        expr_id: ExpressionId,
    ) -> Result<Value, String> {
        // Array/index access in expression context (rvalue) - load from computed address
        let array_addr = self.lower_lvalue_expression(array)?;
        let index_value = self.lower_expression(index)?;

        // For tuples with constant indices, use the index directly since elements are consecutive
        // For general arrays/pointers, use the index directly (element size scaling would be done in a real system)
        let offset_value = index_value;

        // Query semantic type system for the element type
        let element_type = self.ctx.get_expr_type(expr_id);

        // Calculate the address of the array element
        let element_addr = self
            .state
            .mir_function
            .new_typed_value_id(MirType::pointer(element_type.clone()));
        self.instr().add_instruction(
            Instruction::get_element_ptr(element_addr, array_addr, offset_value)
                .with_comment("Get address of array element".to_string()),
        );

        // Load the value from the element address
        let loaded_value = self
            .state
            .mir_function
            .new_typed_value_id(element_type.clone());

        // Register loaded value as a Value

        // Create comment with index if it's a literal
        let comment = match offset_value {
            Value::Literal(Literal::Integer(idx)) => format!("Load array element [{}]", idx),
            _ => "Load array element".to_string(),
        };

        self.instr().load_with(
            element_type,
            loaded_value,
            Value::operand(element_addr),
            comment,
        );

        Ok(Value::operand(loaded_value))
    }

    pub(super) fn lower_function_call(
        &mut self,
        callee: &Spanned<Expression>,
        args: &[Spanned<Expression>],
        expr_id: ExpressionId,
    ) -> Result<CallResult, String> {
        // First, resolve the callee to a FunctionId
        let func_id = match self.resolve_callee_expression(callee) {
            Ok(id) => id,
            Err(_) => {
                // Function not found - return error value for graceful recovery
                return Ok(CallResult::Single(Value::error()));
            }
        };

        // Lower the arguments
        let mut arg_values = Vec::new();
        for arg in args {
            arg_values.push(self.lower_expression(arg)?);
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

    fn lower_tuple_index_on_call(
        &mut self,
        tuple: &Spanned<Expression>,
        index: usize,
        callee: &Spanned<Expression>,
        args: &[Spanned<Expression>],
    ) -> Result<Value, String> {
        // Get the expression ID for the function call
        let func_expr_id = self
            .ctx
            .semantic_index
            .expression_id_by_span(tuple.span())
            .ok_or_else(|| "No ExpressionId for function call in TupleIndex".to_string())?;

        // Lower the function call
        match self.lower_function_call(callee, args, func_expr_id)? {
            CallResult::Single(value) => {
                // Check if it's an error value - if so, return it for graceful recovery
                if matches!(value, Value::Error) {
                    return Ok(value);
                }
                Err("Cannot index a non-tuple value".to_string())
            }
            CallResult::Tuple(values) => {
                // Directly return the indexed value
                if let Some(value) = values.get(index) {
                    Ok(*value)
                } else {
                    Err(format!("Tuple index {} out of bounds", index))
                }
            }
        }
    }

    fn lower_struct_literal(
        &mut self,
        fields: &[(Spanned<String>, Spanned<Expression>)],
        expr_id: ExpressionId,
    ) -> Result<Value, String> {
        // NEW: Value-based struct creation
        // Query semantic type system for struct type
        let struct_type = self.ctx.get_expr_type(expr_id);

        // Lower each field to a value
        let mut field_values = Vec::new();
        for (field_name, field_expr) in fields {
            let field_val = self.lower_expression(field_expr)?;
            field_values.push((field_name.value().clone(), field_val));
        }

        // Create the struct using a single MakeStruct instruction
        let struct_dest = self.make_struct(field_values, struct_type);

        Ok(Value::operand(struct_dest))
    }

    fn lower_tuple_literal(
        &mut self,
        elements: &[Spanned<Expression>],
        expr_id: ExpressionId,
    ) -> Result<Value, String> {
        // NEW: Value-based tuple creation
        if elements.is_empty() {
            // Empty tuple - return proper unit value
            return Ok(Value::unit());
        }

        // Lower each element to a value
        let mut element_values = Vec::new();
        for element_expr in elements {
            let element_val = self.lower_expression(element_expr)?;
            element_values.push(element_val);
        }

        // Query semantic type system for the tuple type
        let tuple_type = self.ctx.get_expr_type(expr_id);

        // Create the tuple using a single MakeTuple instruction
        let tuple_dest = self.make_tuple(element_values, tuple_type);

        Ok(Value::operand(tuple_dest))
    }

    fn lower_tuple_index(
        &mut self,
        tuple: &Spanned<Expression>,
        index: usize,
    ) -> Result<Value, String> {
        // NEW: Value-based tuple element extraction
        // Lower the tuple expression to get a value
        let tuple_val = self.lower_expression(tuple)?;

        // Get the semantic type of the tuple to determine element types
        let tuple_expr_id = self
            .ctx
            .semantic_index
            .expression_id_by_span(tuple.span())
            .ok_or_else(|| "No ExpressionId for tuple in TupleIndex".to_string())?;

        // Get the MIR type of the tuple
        let tuple_mir_type = self.ctx.get_expr_type(tuple_expr_id);

        // Get element type
        let element_mir_type = match &tuple_mir_type {
            MirType::Tuple(types) => types
                .get(index)
                .ok_or_else(|| format!("Tuple index {} out of bounds", index))?
                .clone(),
            _ => return Err("TupleIndex on non-tuple type".to_string()),
        };

        // Extract the element using ExtractTupleElement instruction
        let element_dest = self.extract_tuple_element(tuple_val, index, element_mir_type);

        Ok(Value::operand(element_dest))
    }
}
