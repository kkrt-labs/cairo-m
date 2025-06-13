//! # Control Flow Validation
//!
//! This module implements control-flow validation rules for Cairo-M:
//! - **Unreachable code detection**: Identifies statements that appear after statements that
//!   unconditionally terminate a block, like `return` or an `if-else` where both branches
//!   terminate.
//! - **Missing return detection**: Ensures that every execution path of a function that
//!   is expected to return a value *does* return a value.
//!
//! # Implementation Notes
//!
//! The validator performs two separate analysis passes over each function's AST:
//! 1.  **Unreachable Code Analysis**: A pass that identifies statements following a "hard"
//!     terminating statement. This analysis is recursive and populates diagnostics for any
//!     unreachable statements it finds.
//! 2.  **Missing Return Analysis**: A separate pass that determines if a function with a
//!     non-unit return type guarantees a return value on all paths. If not
//!     all paths are covered, a `MissingReturn` diagnostic is emitted.
//!
use crate::db::SemanticDb;
use crate::definition::DefinitionKind;
use crate::validation::Validator;
use crate::{File, SemanticIndex};
use cairo_m_compiler_diagnostics::Diagnostic;
use cairo_m_compiler_parser::parser::{
    parse_program, FunctionDef, Spanned, Statement, TopLevelItem,
};

/// Validator for control-flow–related semantic rules.
///
/// This validator currently catches unreachable code and functions that do not
/// return on all paths when a return value is required.
pub struct ControlFlowValidator;

impl Validator for ControlFlowValidator {
    fn validate(&self, db: &dyn SemanticDb, file: File, index: &SemanticIndex) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Get the parsed module to access the AST.
        let parsed_program = parse_program(db, file);
        if !parsed_program.diagnostics.is_empty() {
            panic!("Got unexpected parse errors");
        }
        let parsed_module = parsed_program.module;

        // Analyse each function's control-flow.
        for (_def_idx, definition) in index.all_definitions() {
            if let DefinitionKind::Function(_) = &definition.kind {
                self.analyze_function_control_flow(
                    db,
                    file,
                    &parsed_module,
                    &definition.name,
                    &mut diagnostics,
                );
            }
        }

        diagnostics
    }

    fn name(&self) -> &'static str {
        "ControlFlowValidator"
    }
}

impl ControlFlowValidator {
    /// Analyse the control-flow of a specific function, adding diagnostics as needed.
    fn analyze_function_control_flow(
        &self,
        db: &dyn SemanticDb,
        file: File,
        parsed_module: &cairo_m_compiler_parser::parser::ParsedModule,
        function_name: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Find the function definition in the AST.
        if let Some(function_def) = self.find_function_in_module(parsed_module, function_name) {
            // Pass 1: Unreachable code analysis.
            Self::analyze_for_unreachable_code_in_sequence(
                db,
                file,
                &function_def.body,
                diagnostics,
            );

            // Pass 2: Missing-return analysis.
            if !Self::body_returns_on_all_paths(&function_def.body) {
                diagnostics.push(Diagnostic::missing_return(
                    file.file_path(db).to_string(),
                    function_name,
                    function_def.name.span(),
                ));
            }
        }
    }

    /// Locate a function definition by name in the parsed module.
    fn find_function_in_module<'a>(
        &self,
        parsed_module: &'a cairo_m_compiler_parser::parser::ParsedModule,
        function_name: &str,
    ) -> Option<&'a FunctionDef> {
        for item in parsed_module.items() {
            match item {
                TopLevelItem::Function(func_spanned) => {
                    if func_spanned.value().name.value() == function_name {
                        return Some(func_spanned.value());
                    }
                }
                TopLevelItem::Namespace(namespace_spanned) => {
                    // Recursively search namespaces.
                    let namespace = namespace_spanned.value();
                    for namespace_item in &namespace.body {
                        if let TopLevelItem::Function(func_spanned) = namespace_item
                            && func_spanned.value().name.value() == function_name
                        {
                            return Some(func_spanned.value());
                        }
                    }
                }
                _ => {} // Ignore other top-level items.
            }
        }
        None
    }

    // ---------------------------------------------------------------------
    // Unreachable-code analysis
    // ---------------------------------------------------------------------

    /// Analyse a sequence of statements for unreachable code.
    /// Returns `true` if the sequence is guaranteed to terminate.
    fn analyze_for_unreachable_code_in_sequence(
        db: &dyn SemanticDb,
        file: File,
        statements: &[Spanned<Statement>],
        diagnostics: &mut Vec<Diagnostic>,
    ) -> bool {
        let mut path_has_terminated = false;
        for stmt_spanned in statements {
            if path_has_terminated {
                let statement_type = Self::statement_type_name(stmt_spanned.value());
                diagnostics.push(Diagnostic::unreachable_code(
                    file.file_path(db).to_string(),
                    statement_type,
                    stmt_spanned.span(),
                ));
            }

            // Recurse to find nested unreachable code, even if this statement is already unreachable.
            let current_statement_terminates = Self::analyze_for_unreachable_code_in_statement(
                db,
                file,
                stmt_spanned,
                diagnostics,
            );

            if !path_has_terminated {
                path_has_terminated = current_statement_terminates;
            }
        }
        path_has_terminated
    }

    /// Analyse a single statement for unreachable code and check if it terminates.
    fn analyze_for_unreachable_code_in_statement(
        db: &dyn SemanticDb,
        file: File,
        stmt: &Spanned<Statement>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> bool {
        match stmt.value() {
            Statement::Return { .. } => true,
            Statement::Block(body) => {
                Self::analyze_for_unreachable_code_in_sequence(db, file, body, diagnostics)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                let then_terminates = Self::analyze_for_unreachable_code_in_statement(
                    db,
                    file,
                    then_block,
                    diagnostics,
                );
                let else_terminates = else_block.as_ref().is_some_and(|eb| {
                    Self::analyze_for_unreachable_code_in_statement(db, file, eb, diagnostics)
                });
                then_terminates && else_terminates
            }
            // Other statements do not terminate control flow for this analysis.
            _ => false,
        }
    }

    // ---------------------------------------------------------------------
    // Missing-return analysis
    // ---------------------------------------------------------------------

    /// Returns `true` if the function body guarantees a return on all paths.
    fn body_returns_on_all_paths(statements: &[Spanned<Statement>]) -> bool {
        if statements.is_empty() {
            return false;
        }

        // Check for an early, hard return in all but the last statement.
        for stmt in &statements[..statements.len() - 1] {
            if Self::statement_guarantees_hard_return(stmt) {
                return true;
            }
        }

        // If no early return, the outcome depends on the last statement providing a value.
        Self::statement_provides_return_value(statements.last().unwrap())
    }

    /// Checks if a statement can provide a return value, only explicitly (`return`).
    fn statement_provides_return_value(stmt: &Spanned<Statement>) -> bool {
        match stmt.value() {
            Statement::Return { .. } => true,
            Statement::Block(body) => Self::body_returns_on_all_paths(body),
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                let then_returns = Self::statement_provides_return_value(then_block);
                // An `if` must have an `else` to guarantee a return.
                let else_returns = else_block
                    .as_ref()
                    .is_some_and(|eb| Self::statement_provides_return_value(eb));
                then_returns && else_returns
            }
            _ => false, // `let`, `const`, `assign`, and `expression` do not provide return values.
        }
    }

    /// Checks if a statement guarantees an *explicit* `return`. Used for statements
    /// that are not the last in a block. Expressions do not count.
    fn statement_guarantees_hard_return(stmt: &Spanned<Statement>) -> bool {
        match stmt.value() {
            Statement::Return { .. } => true,
            Statement::Block(body) => {
                // A block has a hard return if any of its statements has one.
                body.iter().any(Self::statement_guarantees_hard_return)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                let then_returns = Self::statement_guarantees_hard_return(then_block);
                let else_returns = else_block
                    .as_ref()
                    .is_some_and(|eb| Self::statement_guarantees_hard_return(eb));
                then_returns && else_returns
            }
            _ => false, // let, const, assign, and expression are not hard returns.
        }
    }

    /// Return a static human-readable name for a statement type.
    const fn statement_type_name(stmt: &Statement) -> &'static str {
        match stmt {
            Statement::Let { .. } => "variable declaration",
            Statement::Local { .. } => "local variable declaration",
            Statement::Const(_) => "constant declaration",
            Statement::Assignment { .. } => "assignment",
            Statement::Return { .. } => "return statement",
            Statement::If { .. } => "if statement",
            Statement::Expression(_) => "expression statement",
            Statement::Block(_) => "block",
        }
    }
}
