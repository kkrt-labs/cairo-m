//! # MirBuilder
//!
//! This module contains the main builder struct for constructing MIR functions
//! from the semantic AST. The MirBuilder maintains state during the lowering
//! process and provides core infrastructure for instruction generation.

use cairo_m_compiler_parser::parser::{Expression, Spanned};
use cairo_m_compiler_semantic::db::Crate;
use cairo_m_compiler_semantic::definition::{Definition, DefinitionKind};
use cairo_m_compiler_semantic::semantic_index::{DefinitionId, SemanticIndex};
use cairo_m_compiler_semantic::type_resolution::{
    definition_semantic_type, expression_semantic_type, resolve_ast_type,
};
use cairo_m_compiler_semantic::types::TypeData;
use cairo_m_compiler_semantic::{module_semantic_index, File, SemanticDb};
use rustc_hash::FxHashMap;

use crate::{
    BasicBlock, BasicBlockId, FunctionId, Instruction, MirDefinitionId, MirFunction, MirType,
    Terminator, Value, ValueId,
};

/// A builder that constructs a `MirFunction` from a semantic AST function definition
///
/// The `MirBuilder` maintains state for the function currently being built and provides
/// methods for lowering different AST constructs into MIR instructions and terminators.
pub struct MirBuilder<'a, 'db> {
    pub(super) db: &'db dyn SemanticDb,
    pub(super) file: File,
    pub(super) crate_id: Crate,
    pub(super) semantic_index: &'a SemanticIndex,
    /// Global map from function DefinitionId to MIR FunctionId for call resolution
    pub(super) function_mapping: &'a FxHashMap<DefinitionId<'db>, (&'a Definition, FunctionId)>,
    /// Precomputed file ID for efficient MirDefinitionId creation
    pub(super) file_id: u64,
    /// The DefinitionId of the function being lowered (for type information)
    pub(super) function_def_id: Option<DefinitionId<'db>>,

    // State for the function currently being built
    pub(super) mir_function: MirFunction,
    pub(super) current_block_id: BasicBlockId,
    /// Local map from variable DefinitionId to its MIR ValueId
    pub(super) definition_to_value: FxHashMap<MirDefinitionId, ValueId>,
    /// Becomes true when a terminator like `return` is encountered.
    pub(super) is_terminated: bool,
    /// Stack of loop contexts for break/continue handling
    /// Each entry contains (continue_target_block, loop_exit_block)
    /// - continue_target: where 'continue' jumps (header for while/loop, step for for)
    /// - loop_exit: where 'break' jumps
    pub(super) loop_stack: Vec<(BasicBlockId, BasicBlockId)>,
}

/// Represents the result of lowering a function call
pub enum CallResult {
    /// Single return value
    Single(Value),
    /// Multiple return values (tuple)
    Tuple(Vec<Value>),
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

        Self {
            db,
            file,
            crate_id,
            semantic_index,
            function_mapping,
            mir_function,
            current_block_id: entry_block,
            definition_to_value: FxHashMap::default(),
            file_id,
            function_def_id: None,
            is_terminated: false,
            loop_stack: Vec::new(),
        }
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
        let imported_index =
            module_semantic_index(self.db, self.crate_id, imported_module_name.to_string()).ok()?;

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
        let imported_file = *self.crate_id.modules(self.db).get(imported_module_name)?;

        // Create the correct DefinitionId for the imported function
        let func_def_id = DefinitionId::new(self.db, imported_file, imported_def_idx);

        // Lookup in function_mapping to get the FunctionId
        self.function_mapping
            .get(&func_def_id)
            .map(|(_, func_id)| *func_id)
    }

    /// Converts a parser BinaryOp to MIR BinaryOp, selecting U32 variant if operands are U32 types
    pub fn convert_binary_op(
        &self,
        op: cairo_m_compiler_parser::parser::BinaryOp,
        left_expr: &Spanned<Expression>,
        right_expr: &Spanned<Expression>,
    ) -> crate::BinaryOp {
        use crate::BinaryOp as MirOp;
        use cairo_m_compiler_parser::parser::BinaryOp as ParserOp;

        // Get the expression IDs for the operands
        let left_expr_id = self
            .semantic_index
            .expression_id_by_span(left_expr.span())
            .expect("Left expression should have an ID");
        let right_expr_id = self
            .semantic_index
            .expression_id_by_span(right_expr.span())
            .expect("Right expression should have an ID");

        // Check if both operands have U32 type
        let operand_type = {
            let left_type =
                expression_semantic_type(self.db, self.crate_id, self.file, left_expr_id, None);
            let right_type =
                expression_semantic_type(self.db, self.crate_id, self.file, right_expr_id, None);

            // Verify operands have the same type - this should be guaranteed by semantic analysis
            let left_type_data = left_type.data(self.db);
            let right_type_data = right_type.data(self.db);

            if left_type_data != right_type_data {
                panic!(
                    "MIR: Binary op operands must have the same type, got {:?} and {:?}",
                    left_type_data, right_type_data
                );
            }

            left_type_data
        };

        // Convert parser op to MIR op, selecting U32 variant if needed
        match (op, operand_type.clone()) {
            (ParserOp::Add, TypeData::Felt) => MirOp::Add,
            (ParserOp::Sub, TypeData::Felt) => MirOp::Sub,
            (ParserOp::Mul, TypeData::Felt) => MirOp::Mul,
            (ParserOp::Div, TypeData::Felt) => MirOp::Div,
            (ParserOp::Eq, TypeData::Felt) => MirOp::Eq,
            (ParserOp::Neq, TypeData::Felt) => MirOp::Neq,
            (ParserOp::LessEqual, TypeData::Felt) => MirOp::LessEqual,
            (ParserOp::Less, TypeData::Felt) => MirOp::Less,
            (ParserOp::Greater, TypeData::Felt) => MirOp::Greater,
            (ParserOp::GreaterEqual, TypeData::Felt) => MirOp::GreaterEqual,
            //
            (ParserOp::Add, TypeData::U32) => MirOp::U32Add,
            (ParserOp::Sub, TypeData::U32) => MirOp::U32Sub,
            (ParserOp::Mul, TypeData::U32) => MirOp::U32Mul,
            (ParserOp::Div, TypeData::U32) => MirOp::U32Div,
            (ParserOp::Eq, TypeData::U32) => MirOp::U32Eq,
            (ParserOp::Neq, TypeData::U32) => MirOp::U32Neq,
            (ParserOp::Less, TypeData::U32) => MirOp::U32Less,
            (ParserOp::Greater, TypeData::U32) => MirOp::U32Greater,
            (ParserOp::LessEqual, TypeData::U32) => MirOp::U32LessEqual,
            (ParserOp::GreaterEqual, TypeData::U32) => MirOp::U32GreaterEqual,
            //
            (ParserOp::Eq, TypeData::Bool) => MirOp::Eq,
            (ParserOp::Neq, TypeData::Bool) => MirOp::Neq,
            (ParserOp::And, TypeData::Bool) => MirOp::And,
            (ParserOp::Or, TypeData::Bool) => MirOp::Or,
            _ => panic!(
                "MIR: Unsupported binary op {:?} with type {:?}",
                op, operand_type
            ),
        }
    }

    pub fn current_block_mut(&mut self) -> &mut BasicBlock {
        self.mir_function
            .basic_blocks
            .get_mut(self.current_block_id)
            .expect("Current block should exist")
    }

    pub fn current_block(&self) -> &BasicBlock {
        self.mir_function
            .basic_blocks
            .get(self.current_block_id)
            .expect("Current block should exist")
    }

    pub fn add_instruction(&mut self, instruction: Instruction) {
        let block = self.current_block_mut();
        block.push_instruction(instruction);
    }

    pub fn terminate_current_block(&mut self, terminator: Terminator) {
        let block = self.current_block_mut();
        block.set_terminator(terminator);
        self.is_terminated = true;
    }

    /// Get the return types of the function being lowered
    ///
    /// This retrieves the function's semantic type and extracts the return type information.
    pub fn get_function_return_types(&self, func_def_id: DefinitionId<'db>) -> Vec<MirType> {
        let semantic_type = definition_semantic_type(self.db, self.crate_id, func_def_id);
        let type_data = semantic_type.data(self.db);

        if let TypeData::Function(sig_id) = type_data {
            let return_type = sig_id.return_type(self.db);
            // Convert semantic return type to MIR type
            let mir_type = MirType::from_semantic_type(self.db, return_type);

            // If the return type is a tuple, expand it to individual types
            if let MirType::Tuple(types) = mir_type {
                types
            } else if matches!(mir_type, MirType::Unit) {
                // Unit type means no return values
                vec![]
            } else {
                // Single return value
                vec![mir_type]
            }
        } else {
            panic!("Function definition should have function type");
        }
    }

    pub fn convert_definition_id(&self, def_id: DefinitionId) -> MirDefinitionId {
        MirDefinitionId {
            definition_index: def_id.id_in_file(self.db).index(),
            file_id: self.file_id,
        }
    }

    pub fn get_function_signature(
        &self,
        func_id: FunctionId,
    ) -> Result<(Vec<MirType>, Vec<MirType>), String> {
        // Find the Definition for this FunctionId by searching through function_mapping
        let mut func_def = None;
        for (def_id, (def, fid)) in self.function_mapping {
            if *fid == func_id {
                func_def = Some((def_id, def));
                break;
            }
        }

        let (def_id, def) =
            func_def.ok_or_else(|| "Function definition not found in mapping".to_string())?;

        // Extract the FunctionDefRef from the Definition
        let func_ref = match &def.kind {
            DefinitionKind::Function(func_ref) => func_ref,
            _ => return Err("Definition is not a function".to_string()),
        };

        // Convert parameter types from AST to MIR types
        let mut param_types = Vec::new();
        for (_, param_type_ast) in &func_ref.params_ast {
            let semantic_type = resolve_ast_type(
                self.db,
                self.crate_id,
                def_id.file(self.db),
                param_type_ast.clone(),
                def.scope_id,
            );
            param_types.push(MirType::from_semantic_type(self.db, semantic_type));
        }

        // Convert return type from AST to MIR type
        let return_semantic_type = resolve_ast_type(
            self.db,
            self.crate_id,
            def_id.file(self.db),
            func_ref.return_type_ast.clone(),
            def.scope_id,
        );

        // Handle return types - could be unit (empty tuple), single, or tuple
        let return_types = match return_semantic_type.data(self.db) {
            cairo_m_compiler_semantic::types::TypeData::Tuple(element_types)
                if element_types.is_empty() =>
            {
                vec![]
            }
            cairo_m_compiler_semantic::types::TypeData::Tuple(element_types) => element_types
                .iter()
                .map(|t| MirType::from_semantic_type(self.db, *t))
                .collect(),
            _ => vec![MirType::from_semantic_type(self.db, return_semantic_type)],
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
                    .semantic_index
                    .expression_id_by_span(callee.span())
                    .ok_or_else(|| "No ExpressionId found for callee".to_string())?;
                let callee_expr_info = self
                    .semantic_index
                    .expression(callee_expr_id)
                    .ok_or_else(|| "No ExpressionInfo for callee".to_string())?;

                if let Some((local_def_idx, local_def)) = self
                    .semantic_index
                    .resolve_name_to_definition(func_name.value(), callee_expr_info.scope_id)
                {
                    match &local_def.kind {
                        DefinitionKind::Function(_) => {
                            // Local function
                            let func_def_id = DefinitionId::new(self.db, self.file, local_def_idx);
                            if let Some((_, func_id)) = self.function_mapping.get(&func_def_id) {
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
}
