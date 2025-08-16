//! # MirBuilder
//!
//! This module contains the main builder struct for constructing MIR functions
//! from the semantic AST. The MirBuilder maintains state during the lowering
//! process and provides core infrastructure for instruction generation.

use std::cell::RefCell;

use cairo_m_compiler_parser::parser::{Expression, Spanned};
use cairo_m_compiler_semantic::db::Crate;
use cairo_m_compiler_semantic::definition::{Definition, DefinitionKind};
use cairo_m_compiler_semantic::semantic_index::{DefinitionId, ExpressionId, SemanticIndex};
use cairo_m_compiler_semantic::type_resolution::{
    definition_semantic_type, expression_semantic_type, resolve_ast_type,
};
use cairo_m_compiler_semantic::types::TypeData;
use cairo_m_compiler_semantic::{module_semantic_index, File, SemanticDb};
use rustc_hash::FxHashMap;

use crate::{
    BasicBlockId, BinaryOp, CfgBuilder, FunctionId, InstrBuilder, Instruction, MirDefinitionId,
    MirFunction, MirType, Value, ValueId,
};

/// Immutable compilation context shared across lowering
///
/// This contains all the read-only data needed during MIR lowering,
/// including the semantic database, indices, and caches for improved performance.
pub struct LoweringContext<'a, 'db> {
    pub(super) db: &'db dyn SemanticDb,
    pub(super) file: File,
    pub(super) crate_id: Crate,
    pub(super) semantic_index: &'a SemanticIndex,
    /// Global map from function DefinitionId to MIR FunctionId for call resolution
    pub(super) function_mapping: &'a FxHashMap<DefinitionId<'db>, (&'a Definition, FunctionId)>,
    /// Reverse mapping from FunctionId to DefinitionId for O(1) signature lookups
    pub(super) function_id_to_def:
        RefCell<FxHashMap<FunctionId, (DefinitionId<'db>, &'a Definition)>>,
    /// Precomputed file ID for efficient MirDefinitionId creation
    pub(super) file_id: u64,

    // Caches to improve performance
    /// Cache of expression types to avoid repeated semantic queries
    pub(super) expr_type_cache: RefCell<FxHashMap<ExpressionId, MirType>>,
}

/// Mutable state for the function being built
///
/// This contains all the mutable state needed during function construction,
/// including the function itself, current block tracking, and variable mappings.
pub struct MirState<'db> {
    /// The MIR function being constructed
    pub(super) mir_function: MirFunction,
    /// The current basic block being populated with instructions
    pub(super) current_block_id: BasicBlockId,
    /// Local map from variable DefinitionId to its MIR ValueId
    pub(super) definition_to_value: FxHashMap<MirDefinitionId, ValueId>,
    /// The DefinitionId of the function being lowered (for type information)
    pub(super) function_def_id: Option<DefinitionId<'db>>,
    /// Becomes true when a terminator like `return` is encountered.
    pub(super) is_terminated: bool,
    /// Stack of loop contexts for break/continue handling
    /// Each entry contains (continue_target_block, loop_exit_block)
    /// - continue_target: where 'continue' jumps (header for while/loop, step for for)
    /// - loop_exit: where 'break' jumps
    pub(super) loop_stack: Vec<(BasicBlockId, BasicBlockId)>,
}

/// A builder that constructs a `MirFunction` from a semantic AST function definition
///
/// The `MirBuilder` combines the immutable context with mutable state and provides
/// methods for lowering different AST constructs into MIR instructions and terminators.
pub struct MirBuilder<'a, 'db> {
    /// Immutable compilation context
    pub(super) ctx: LoweringContext<'a, 'db>,
    /// Mutable function state
    pub(super) state: MirState<'db>,
}

/// Represents the result of lowering a function call
pub enum CallResult {
    /// Single return value
    Single(Value),
    /// Multiple return values (tuple)
    Tuple(Vec<Value>),
}

impl<'a, 'db> LoweringContext<'a, 'db> {
    /// Get or compute the MIR type for an expression
    pub fn get_expr_type(&self, expr_id: ExpressionId) -> MirType {
        let mut cache = self.expr_type_cache.borrow_mut();
        cache
            .entry(expr_id)
            .or_insert_with(|| {
                let sem_type =
                    expression_semantic_type(self.db, self.crate_id, self.file, expr_id, None);
                MirType::from_semantic_type(self.db, sem_type)
            })
            .clone()
    }
}

impl<'a, 'db> MirBuilder<'a, 'db> {
    pub fn new(
        db: &'db dyn SemanticDb,
        file: File,
        semantic_index: &'a SemanticIndex,
        function_mapping: &'a FxHashMap<DefinitionId<'db>, (&'a Definition, FunctionId)>,
        file_id: u64,
        crate_id: Crate,
    ) -> Self {
        // Create a placeholder function - will be filled in during lowering
        let mir_function = MirFunction::new(String::new());
        let entry_block = mir_function.entry_block;

        // Build reverse mapping for O(1) function signature lookups
        let mut function_id_to_def = FxHashMap::default();
        for (def_id, (def, func_id)) in function_mapping.iter() {
            function_id_to_def.insert(*func_id, (*def_id, *def));
        }

        let ctx = LoweringContext {
            db,
            file,
            crate_id,
            semantic_index,
            function_mapping,
            function_id_to_def: RefCell::new(function_id_to_def),
            file_id,
            expr_type_cache: RefCell::new(FxHashMap::default()),
        };

        let state = MirState {
            mir_function,
            current_block_id: entry_block,
            definition_to_value: FxHashMap::default(),
            function_def_id: None,
            is_terminated: false,
            loop_stack: Vec::new(),
        };

        Self { ctx, state }
    }

    /// Resolves an imported function to its FunctionId in the crate
    ///
    /// Follows the import chain: module_name.function_name -> FunctionId
    pub fn resolve_imported_function(
        &self,
        imported_module_name: &str,
        function_name: &str,
    ) -> Option<FunctionId> {
        // Get the crate's semantic index
        let imported_index = module_semantic_index(
            self.ctx.db,
            self.ctx.crate_id,
            imported_module_name.to_string(),
        )
        .ok()?;

        // Get imported module's root scope
        let imported_root = imported_index.root_scope()?;

        // Resolve the actual function definition in the imported module
        let (imported_def_idx, imported_def) =
            imported_index.resolve_name_to_definition(function_name, imported_root)?;

        // Verify it's actually a function
        if !matches!(imported_def.kind, DefinitionKind::Function(_)) {
            return None;
        }

        // Get the imported file
        let imported_file = *self
            .ctx
            .crate_id
            .modules(self.ctx.db)
            .get(imported_module_name)?;

        // Create the correct DefinitionId for the imported function
        let func_def_id = DefinitionId::new(self.ctx.db, imported_file, imported_def_idx);

        // Lookup in function_mapping to get the FunctionId
        self.ctx
            .function_mapping
            .get(&func_def_id)
            .map(|(_, func_id)| *func_id)
    }

    // ================================================================================
    // CFG Operations - All delegated through CfgBuilder
    // ================================================================================

    /// Creates a CfgBuilder for the current function
    pub(super) fn cfg(&mut self) -> CfgBuilder {
        CfgBuilder::new(&mut self.state.mir_function, self.state.current_block_id)
    }

    /// Creates a new block and returns its ID
    pub fn create_block(&mut self) -> BasicBlockId {
        self.cfg().new_block(None)
    }

    /// Switches to a different block
    pub fn switch_to_block(&mut self, block_id: BasicBlockId) {
        let mut cfg = self.cfg();
        let state = cfg.switch_to_block(block_id);
        self.state.current_block_id = state.current_block_id;
        self.state.is_terminated = state.is_terminated;
    }

    /// Terminates the current block with a jump to the target
    pub fn terminate_with_jump(&mut self, target: BasicBlockId) {
        let state = self.cfg().terminate_with_jump(target);
        self.state.is_terminated = state.is_terminated;
    }

    /// Terminates the current block with a conditional branch
    pub fn terminate_with_branch(
        &mut self,
        condition: Value,
        then_block: BasicBlockId,
        else_block: BasicBlockId,
    ) {
        let state = self
            .cfg()
            .terminate_with_branch(condition, then_block, else_block);
        self.state.is_terminated = state.is_terminated;
    }

    /// Terminates the current block with a return
    pub fn terminate_with_return(&mut self, values: Vec<Value>) {
        let state = self.cfg().terminate_with_return(values);
        self.state.is_terminated = state.is_terminated;
    }

    /// Creates blocks for an if statement
    pub fn create_if_blocks(&mut self) -> (BasicBlockId, BasicBlockId, BasicBlockId) {
        self.cfg().create_if_blocks()
    }

    /// Creates blocks for a loop
    pub fn create_loop_blocks(&mut self) -> (BasicBlockId, BasicBlockId, BasicBlockId) {
        self.cfg().create_loop_blocks()
    }

    /// Creates blocks for a for loop
    pub fn create_for_loop_blocks(
        &mut self,
    ) -> (BasicBlockId, BasicBlockId, BasicBlockId, BasicBlockId) {
        self.cfg().create_for_loop_blocks()
    }

    // ================================================================================
    // Instruction Operations - Delegated through InstrBuilder
    // ================================================================================

    /// Creates an InstrBuilder for the current block
    pub(super) fn instr(&mut self) -> InstrBuilder {
        InstrBuilder::new(&mut self.state.mir_function, self.state.current_block_id)
    }

    /// Add an instruction directly (legacy support - prefer using instr() methods)
    pub fn add_instruction(&mut self, instruction: Instruction) {
        self.instr().add_instruction(instruction);
    }

    /// Allocate frame space for a variable
    pub fn alloc_frame(&mut self, ty: MirType) -> ValueId {
        self.instr().alloc_frame(ty)
    }

    /// Create a binary operation with automatic destination
    pub fn binary_op_auto(
        &mut self,
        op: BinaryOp,
        lhs: Value,
        rhs: Value,
        result_type: MirType,
    ) -> ValueId {
        self.instr().binary_op(op, lhs, rhs, result_type)
    }

    /// Load a value with automatic destination
    pub fn load_auto(&mut self, src: Value, ty: MirType) -> ValueId {
        self.instr().load(src, ty)
    }

    /// Store a value
    pub fn store_value(&mut self, dest: Value, value: Value, ty: MirType) {
        self.instr().store(dest, value, ty);
    }

    /// Get element pointer
    pub fn get_element_ptr_auto(
        &mut self,
        base: Value,
        offset: Value,
        elem_type: MirType,
    ) -> ValueId {
        let dest = self
            .state
            .mir_function
            .new_typed_value_id(MirType::pointer(elem_type));
        self.instr().get_element_ptr_to(dest, base, offset);
        dest
    }

    /// Check if the current block is terminated
    pub fn is_current_block_terminated(&mut self) -> bool {
        self.cfg().is_terminated()
    }

    /// Get the return types of the function being lowered
    ///
    /// This retrieves the function's semantic type and extracts the return type information.
    pub fn get_function_return_types(
        &self,
        func_def_id: DefinitionId<'db>,
    ) -> Result<Vec<MirType>, String> {
        let semantic_type = definition_semantic_type(self.ctx.db, self.ctx.crate_id, func_def_id);
        let type_data = semantic_type.data(self.ctx.db);

        if let TypeData::Function(sig_id) = type_data {
            let return_type = sig_id.return_type(self.ctx.db);
            // Convert semantic return type to MIR type
            let mir_type = MirType::from_semantic_type(self.ctx.db, return_type);

            // If the return type is a tuple, expand it to individual types
            Ok(if let MirType::Tuple(types) = mir_type {
                types
            } else if matches!(mir_type, MirType::Unit) {
                // Unit type means no return values
                vec![]
            } else {
                // Single return value
                vec![mir_type]
            })
        } else {
            Err(
                "Internal Compiler Error: Function definition should have function type"
                    .to_string(),
            )
        }
    }

    pub fn convert_definition_id(&self, def_id: DefinitionId) -> MirDefinitionId {
        MirDefinitionId {
            definition_index: def_id.id_in_file(self.ctx.db).index(),
            file_id: self.ctx.file_id,
        }
    }

    pub fn get_function_signature(
        &self,
        func_id: FunctionId,
    ) -> Result<(Vec<MirType>, Vec<MirType>), String> {
        // Use reverse mapping for O(1) lookup instead of linear scan
        let cache = self.ctx.function_id_to_def.borrow();
        let (def_id, def) = cache
            .get(&func_id)
            .ok_or_else(|| "Function definition not found in mapping".to_string())?;

        // Extract the FunctionDefRef from the Definition
        let func_ref = match &def.kind {
            DefinitionKind::Function(func_ref) => func_ref,
            _ => return Err("Definition is not a function".to_string()),
        };

        // Convert parameter types from AST to MIR types
        let mut param_types = Vec::new();
        for (_, param_type_ast) in &func_ref.params_ast {
            let semantic_type = resolve_ast_type(
                self.ctx.db,
                self.ctx.crate_id,
                def_id.file(self.ctx.db),
                param_type_ast.clone(),
                def.scope_id,
            );
            param_types.push(MirType::from_semantic_type(self.ctx.db, semantic_type));
        }

        // Convert return type from AST to MIR type
        let return_semantic_type = resolve_ast_type(
            self.ctx.db,
            self.ctx.crate_id,
            def_id.file(self.ctx.db),
            func_ref.return_type_ast.clone(),
            def.scope_id,
        );

        // Handle return types - could be unit (empty tuple), single, or tuple
        let return_types = match return_semantic_type.data(self.ctx.db) {
            cairo_m_compiler_semantic::types::TypeData::Tuple(element_types)
                if element_types.is_empty() =>
            {
                vec![]
            }
            cairo_m_compiler_semantic::types::TypeData::Tuple(element_types) => element_types
                .iter()
                .map(|t| MirType::from_semantic_type(self.ctx.db, *t))
                .collect(),
            _ => vec![MirType::from_semantic_type(
                self.ctx.db,
                return_semantic_type,
            )],
        };

        Ok((param_types, return_types))
    }

    /// Resolves a callee expression to a FunctionId
    /// Supports:
    /// - Simple identifiers (foo)
    /// - Member access for imports (module.foo)
    pub fn resolve_callee_expression(
        &self,
        callee: &Spanned<Expression>,
    ) -> Result<FunctionId, String> {
        match callee.value() {
            Expression::Identifier(func_name) => {
                // Get the scope for the callee from its expression info
                let callee_expr_id = self
                    .ctx
                    .semantic_index
                    .expression_id_by_span(callee.span())
                    .ok_or_else(|| "No ExpressionId found for callee".to_string())?;
                let callee_expr_info = self
                    .ctx
                    .semantic_index
                    .expression(callee_expr_id)
                    .ok_or_else(|| "No ExpressionInfo for callee".to_string())?;

                if let Some((local_def_idx, local_def)) = self
                    .ctx
                    .semantic_index
                    .resolve_name_to_definition(func_name.value(), callee_expr_info.scope_id)
                {
                    match &local_def.kind {
                        DefinitionKind::Function(_) => {
                            // Local function
                            let func_def_id =
                                DefinitionId::new(self.ctx.db, self.ctx.file, local_def_idx);
                            if let Some((_, func_id)) = self.ctx.function_mapping.get(&func_def_id)
                            {
                                Ok(*func_id)
                            } else {
                                Err(format!(
                                    "Function '{}' not found in mapping",
                                    func_name.value()
                                ))
                            }
                        }
                        DefinitionKind::Use(use_ref) => {
                            // Imported function
                            self.resolve_imported_function(
                                use_ref.imported_module.value(),
                                func_name.value(),
                            )
                            .ok_or_else(|| {
                                format!(
                                    "Failed to resolve imported function '{}'",
                                    func_name.value()
                                )
                            })
                        }
                        _ => Err(format!("'{}' is not a function", func_name.value())),
                    }
                } else {
                    Err(format!("Function '{}' not found", func_name.value()))
                }
            }
            Expression::MemberAccess { object, field } => {
                // Handle module.function pattern
                if let Expression::Identifier(module_name) = object.value() {
                    // This could be an imported module function
                    self.resolve_imported_function(module_name.value(), field.value())
                        .ok_or_else(|| {
                            format!(
                                "Failed to resolve {}.{}",
                                module_name.value(),
                                field.value()
                            )
                        })
                } else {
                    Err("Complex member access callees not yet supported".to_string())
                }
            }
            _ => Err("Unsupported callee expression type".to_string()),
        }
    }

    // ===== Memory Access Helper Methods =====

    /// Load a field from a struct/object
    /// Returns the ValueId of the loaded value
    pub fn load_field(
        &mut self,
        base_addr: Value,
        offset: Value,
        field_type: MirType,
        field_name: &str,
    ) -> ValueId {
        let field_addr = self
            .state
            .mir_function
            .new_typed_value_id(MirType::pointer(field_type.clone()));
        self.instr().add_instruction(
            Instruction::get_element_ptr(field_addr, base_addr, offset)
                .with_comment(format!("Get address of field '{}'", field_name)),
        );

        let loaded_value = self
            .state
            .mir_function
            .new_typed_value_id(field_type.clone());
        self.instr().load_with(
            field_type,
            loaded_value,
            Value::operand(field_addr),
            format!("Load field '{}'", field_name),
        );
        loaded_value
    }

    /// Store a value to a struct field
    pub fn store_field(
        &mut self,
        base_addr: Value,
        offset: Value,
        value: Value,
        field_type: MirType,
        field_name: &str,
    ) {
        let field_addr = self
            .state
            .mir_function
            .new_typed_value_id(MirType::pointer(field_type.clone()));
        self.instr().add_instruction(
            Instruction::get_element_ptr(field_addr, base_addr, offset)
                .with_comment(format!("Get address of field '{}'", field_name)),
        );
        self.instr()
            .store(Value::operand(field_addr), value, field_type);
    }

    /// Load a tuple element by index
    /// Returns the ValueId of the loaded value
    pub fn load_tuple_element(
        &mut self,
        tuple_addr: Value,
        index: usize,
        elem_type: MirType,
    ) -> ValueId {
        let elem_ptr = self
            .state
            .mir_function
            .new_typed_value_id(MirType::pointer(elem_type.clone()));
        self.instr().add_instruction(
            Instruction::get_element_ptr(elem_ptr, tuple_addr, Value::integer(index as i32))
                .with_comment(format!("Get address of tuple element {}", index)),
        );

        let elem_value = self
            .state
            .mir_function
            .new_typed_value_id(elem_type.clone());
        self.instr().load_with(
            elem_type,
            elem_value,
            Value::operand(elem_ptr),
            format!("Load tuple element {}", index),
        );
        elem_value
    }

    /// Store a value to a tuple element
    pub fn store_tuple_element(
        &mut self,
        tuple_addr: Value,
        index: usize,
        value: Value,
        elem_type: MirType,
    ) {
        let elem_ptr = self
            .state
            .mir_function
            .new_typed_value_id(MirType::pointer(elem_type.clone()));
        self.instr().add_instruction(
            Instruction::get_element_ptr(elem_ptr, tuple_addr, Value::integer(index as i32))
                .with_comment(format!("Get address of tuple element {}", index)),
        );
        self.instr()
            .store(Value::operand(elem_ptr), value, elem_type);
    }

    /// Get the address of a field/element (for lvalue expressions)
    /// Returns the ValueId of the address
    pub fn get_element_address(
        &mut self,
        base_addr: Value,
        offset: Value,
        target_type: MirType,
        comment: &str,
    ) -> ValueId {
        let addr = self
            .state
            .mir_function
            .new_typed_value_id(MirType::pointer(target_type));
        self.instr().add_instruction(
            Instruction::get_element_ptr(addr, base_addr, offset).with_comment(comment.to_string()),
        );
        addr
    }

    // ================================================================================
    // Value-Based Aggregate Operations
    // ================================================================================

    /// Create a tuple from a list of values
    /// Returns the ValueId of the new tuple
    pub fn make_tuple(&mut self, elements: Vec<Value>, tuple_type: MirType) -> ValueId {
        let dest = self.state.mir_function.new_typed_value_id(tuple_type);
        self.instr()
            .add_instruction(Instruction::make_tuple(dest, elements));
        dest
    }

    /// Extract an element from a tuple value
    /// Returns the ValueId of the extracted element
    pub fn extract_tuple_element(
        &mut self,
        tuple_val: Value,
        index: usize,
        element_type: MirType,
    ) -> ValueId {
        let dest = self
            .state
            .mir_function
            .new_typed_value_id(element_type.clone());
        self.instr()
            .add_instruction(Instruction::extract_tuple_element(
                dest,
                tuple_val,
                index,
                element_type,
            ));
        dest
    }

    /// Create a struct from field values
    /// Returns the ValueId of the new struct
    pub fn make_struct(&mut self, fields: Vec<(String, Value)>, struct_type: MirType) -> ValueId {
        let dest = self
            .state
            .mir_function
            .new_typed_value_id(struct_type.clone());
        self.instr()
            .add_instruction(Instruction::make_struct(dest, fields, struct_type));
        dest
    }

    /// Extract a field from a struct value
    /// Returns the ValueId of the extracted field
    pub fn extract_struct_field(
        &mut self,
        struct_val: Value,
        field_name: String,
        field_type: MirType,
    ) -> ValueId {
        let dest = self
            .state
            .mir_function
            .new_typed_value_id(field_type.clone());
        self.instr()
            .add_instruction(Instruction::extract_struct_field(
                dest, struct_val, field_name, field_type,
            ));
        dest
    }

    /// Insert a field into a struct value, creating a new struct
    /// Returns the ValueId of the new struct with the field updated
    pub fn insert_field(
        &mut self,
        struct_val: Value,
        field_name: String,
        new_value: Value,
        struct_type: MirType,
    ) -> ValueId {
        let dest = self
            .state
            .mir_function
            .new_typed_value_id(struct_type.clone());
        self.instr().add_instruction(Instruction::insert_field(
            dest,
            struct_val,
            field_name,
            new_value,
            struct_type,
        ));
        dest
    }

    /// Insert an element into a tuple value, creating a new tuple
    /// Returns the ValueId of the new tuple with the element updated
    pub fn insert_tuple(
        &mut self,
        tuple_val: Value,
        index: usize,
        new_value: Value,
        tuple_type: MirType,
    ) -> ValueId {
        let dest = self
            .state
            .mir_function
            .new_typed_value_id(tuple_type.clone());
        self.instr().add_instruction(Instruction::insert_tuple(
            dest, tuple_val, index, new_value, tuple_type,
        ));
        dest
    }

    // ================================================================================
    // Helper Methods - Common Patterns
    // ================================================================================

    /// Get the ExpressionId for a given span
    ///
    /// This is a common pattern used throughout the lowering code
    pub fn expr_id(&self, span: chumsky::prelude::SimpleSpan) -> Result<ExpressionId, String> {
        self.ctx
            .semantic_index
            .expression_id_by_span(span)
            .ok_or_else(|| {
                format!(
                    "Internal Compiler Error: No ExpressionId found for span {:?}",
                    span
                )
            })
    }

    /// Get the MirType for an expression at a given span
    ///
    /// This combines the common pattern of:
    /// 1. Getting the ExpressionId from span
    /// 2. Getting the semantic type from the expression
    /// 3. Converting to MirType
    pub fn expr_mir_type(&self, span: chumsky::prelude::SimpleSpan) -> Result<MirType, String> {
        let expr_id = self.expr_id(span)?;
        let semantic_type =
            expression_semantic_type(self.ctx.db, self.ctx.crate_id, self.ctx.file, expr_id, None);
        Ok(MirType::from_semantic_type(self.ctx.db, semantic_type))
    }

    /// Get both the ExpressionId and MirType for a span
    ///
    /// This is useful when you need both values, avoiding duplicate lookups
    pub fn expr_id_and_type(
        &self,
        span: chumsky::prelude::SimpleSpan,
    ) -> Result<(ExpressionId, MirType), String> {
        let expr_id = self.expr_id(span)?;
        let semantic_type =
            expression_semantic_type(self.ctx.db, self.ctx.crate_id, self.ctx.file, expr_id, None);
        let mir_type = MirType::from_semantic_type(self.ctx.db, semantic_type);
        Ok((expr_id, mir_type))
    }
}
