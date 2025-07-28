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
use cairo_m_compiler_diagnostics::Diagnostic;
use cairo_m_compiler_parser::parser::{FunctionDef, Spanned, Statement, TopLevelItem, parse_file};

use crate::db::{Crate, SemanticDb};
use crate::definition::DefinitionKind;
use crate::validation::Validator;
use crate::{File, SemanticIndex};

/// Validator for control-flow–related semantic rules.
///
/// This validator currently catches unreachable code and functions that do not
/// return on all paths when a return value is required.
pub struct ControlFlowValidator;

impl Validator for ControlFlowValidator {
    fn validate(
        &self,
        db: &dyn SemanticDb,
        _crate_id: Crate,
        file: File,
        index: &SemanticIndex,
        sink: &dyn cairo_m_compiler_diagnostics::DiagnosticSink,
    ) {
        let parsed_program = parse_file(db, file);
        if !parsed_program.diagnostics.is_empty() {
            panic!("Got unexpected parse errors");
        }
        let parsed_module = parsed_program.module;

        // Analyse each function's control-flow in this module only.
        for (_def_idx, definition) in index.all_definitions() {
            if let DefinitionKind::Function(_) = &definition.kind {
                self.analyze_function_control_flow(
                    db,
                    file,
                    &parsed_module,
                    &definition.name,
                    sink,
                );
            }
        }
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
        sink: &dyn cairo_m_compiler_diagnostics::DiagnosticSink,
    ) {
        // Find the function definition in the AST.
        if let Some(function_def) = self.find_function_in_module(parsed_module, function_name) {
            // Pass 1: Unreachable code analysis.
            Self::analyze_for_unreachable_code_in_sequence(
                db,
                file,
                &function_def.body,
                0, // Start with loop depth 0
                sink,
            );

            // Pass 2: Missing-return analysis.
            // Cairo-M requires explicit returns for all functions, including unit-type functions.
            if !Self::body_returns_on_all_paths(&function_def.body) {
                sink.push(Diagnostic::missing_return(
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
        loop_depth: usize,
        sink: &dyn cairo_m_compiler_diagnostics::DiagnosticSink,
    ) -> bool {
        let mut path_has_terminated = false;
        for stmt_spanned in statements {
            if path_has_terminated {
                let statement_type = Self::statement_type_name(stmt_spanned.value());
                sink.push(Diagnostic::unreachable_code(
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
                loop_depth,
                sink,
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
        loop_depth: usize,
        sink: &dyn cairo_m_compiler_diagnostics::DiagnosticSink,
    ) -> bool {
        match stmt.value() {
            Statement::Return { .. } => true,
            Statement::Block(body) => {
                Self::analyze_for_unreachable_code_in_sequence(db, file, body, loop_depth, sink)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                let then_terminates = Self::analyze_for_unreachable_code_in_statement(
                    db, file, then_block, loop_depth, sink,
                );
                let else_terminates = else_block.as_ref().is_some_and(|eb| {
                    Self::analyze_for_unreachable_code_in_statement(db, file, eb, loop_depth, sink)
                });
                then_terminates && else_terminates
            }
            Statement::Loop { body } => {
                // Analyze the loop body for unreachable code
                Self::analyze_for_unreachable_code_in_statement(
                    db,
                    file,
                    body,
                    loop_depth + 1,
                    sink,
                );
                // An infinite loop only terminates control flow if it has no break statements
                !Self::contains_break(body)
            }
            Statement::While { condition: _, body } => {
                // Analyze the loop body for unreachable code
                Self::analyze_for_unreachable_code_in_statement(
                    db,
                    file,
                    body,
                    loop_depth + 1,
                    sink,
                );
                // While loops might not execute at all, so they don't guarantee termination
                false
            }
            Statement::For {
                init,
                condition: _,
                step,
                body,
            } => {
                // 1. Initialization part (may contain returns, etc.)
                Self::analyze_for_unreachable_code_in_statement(db, file, init, loop_depth, sink);

                // 2. Body (inside the loop, so break/continue are valid)
                Self::analyze_for_unreachable_code_in_statement(
                    db,
                    file,
                    body,
                    loop_depth + 1,
                    sink,
                );

                // 3. Step statement (runs after each iteration)
                Self::analyze_for_unreachable_code_in_statement(db, file, step, loop_depth, sink);

                // For loops might not execute at all, so they don't guarantee termination
                false
            }
            Statement::Break => {
                // Check if break is inside a loop
                if loop_depth == 0 {
                    sink.push(Diagnostic::break_outside_loop(
                        file.file_path(db).to_string(),
                        stmt.span(),
                    ));
                }
                // Break terminates the current control flow only when inside a loop
                loop_depth > 0
            }
            Statement::Continue => {
                // Check if continue is inside a loop
                if loop_depth == 0 {
                    sink.push(Diagnostic::continue_outside_loop(
                        file.file_path(db).to_string(),
                        stmt.span(),
                    ));
                }
                // Continue terminates the current control flow only when inside a loop
                loop_depth > 0
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
            Statement::Loop { body } => {
                // Loop statements can provide returns if they contain return statements
                // TODO: This could be improved to check if all break paths lead to returns
                Self::statement_provides_return_value(body)
            }
            Statement::While { .. } => {
                // While loops might not execute, so they can't guarantee a return
                false
            }
            Statement::For { .. } => {
                // For loops might not execute, so they can't guarantee a return
                false
            }
            _ => false, // `let`, `const`, `assign`, `expression`, `break`, and `continue` do not provide return values.
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
            Statement::Loop { body } => {
                // Check if the loop body has a hard return
                Self::statement_guarantees_hard_return(body)
            }
            Statement::While { .. } | Statement::For { .. } => {
                // While and for loops might not execute, so they don't guarantee hard returns
                false
            }
            _ => false, // let, const, assign, expression, break, and continue are not hard returns.
        }
    }

    /// Check if a statement contains any break statements
    fn contains_break(stmt: &Spanned<Statement>) -> bool {
        match stmt.value() {
            Statement::Break => true,
            Statement::Block(statements) => statements.iter().any(Self::contains_break),
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                Self::contains_break(then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|eb| Self::contains_break(eb))
            }
            Statement::Loop { body: _ }
            | Statement::While { body: _, .. }
            | Statement::For { body: _, .. } => {
                // Don't look inside nested loops - their breaks don't affect the outer loop
                false
            }
            _ => false,
        }
    }

    /// Return a static human-readable name for a statement type.
    const fn statement_type_name(stmt: &Statement) -> &'static str {
        match stmt {
            Statement::Let { .. } => "variable declaration",
            Statement::Const(_) => "constant declaration",
            Statement::Assignment { .. } => "assignment",
            Statement::Return { .. } => "return statement",
            Statement::If { .. } => "if statement",
            Statement::Expression(_) => "expression statement",
            Statement::Block(_) => "block",
            Statement::Loop { .. } => "loop statement",
            Statement::While { .. } => "while loop",
            Statement::For { .. } => "for loop",
            Statement::Break => "break statement",
            Statement::Continue => "continue statement",
        }
    }
}
