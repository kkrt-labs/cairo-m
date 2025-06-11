//! # Diagnostic System for Semantic Analysis
//!
//! This module provides the diagnostic infrastructure for reporting semantic errors,
//! warnings, and hints during semantic analysis.

use ariadne::ReportKind;
use chumsky::span::SimpleSpan;
use std::fmt;

/// A diagnostic message from semantic analysis
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: DiagnosticCode,
    pub message: String,
    /// Source span where this diagnostic applies
    pub span: SimpleSpan<usize>,
    /// Optional related spans for additional context
    pub related_spans: Vec<(SimpleSpan<usize>, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl From<ReportKind<'static>> for DiagnosticSeverity {
    fn from(kind: ReportKind<'static>) -> Self {
        match kind {
            ReportKind::Error => Self::Error,
            ReportKind::Warning => Self::Warning,
            ReportKind::Advice => Self::Info,
            ReportKind::Custom(_, _) => Self::Info,
        }
    }
}

impl From<DiagnosticSeverity> for ReportKind<'static> {
    fn from(severity: DiagnosticSeverity) -> Self {
        match severity {
            DiagnosticSeverity::Error => ReportKind::Error,
            DiagnosticSeverity::Warning => ReportKind::Warning,
            DiagnosticSeverity::Info => ReportKind::Advice,
            DiagnosticSeverity::Hint => ReportKind::Advice,
        }
    }
}

impl fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
            Self::Hint => write!(f, "hint"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    // Parse-related errors (0-999)
    LexicalError,
    SyntaxError,
    UnexpectedToken,
    UnexpectedEndOfFile,
    InvalidCharacter,

    // Scope-related errors (1000-1999)
    UndeclaredVariable,
    UnusedVariable,
    DuplicateDefinition,
    UseBeforeDefinition,

    // Type-related errors (2000-2999) - placeholder for future
    TypeMismatch,
    InvalidFieldAccess,
    InvalidIndexAccess,
    InvalidIndexType,
    InvalidStructLiteral,
    InvalidFunctionCall,
    InvalidAssignment,
    InvalidReturnType,
    InvalidTypeDefinition,
    // TODO: Add more type-related diagnostic codes:
    // - InvalidTypeAnnotation
    // - TypeArgumentMismatch
    // - IncompatibleTypes
    // - MissingTypeAnnotation
    // - CyclicTypeDefinition

    // Flow-related errors (3000-3999) - placeholder for future
    UnreachableCode,
    MissingReturn,
    // TODO: Add more control flow diagnostic codes:
    // - DeadCode
    // - InvalidBreak
    // - InvalidContinue
    // - UnreachablePattern

    // TODO: Add more diagnostic categories:
    // - Import/module errors (4000-4999)
    // - Syntax/style warnings (5000-5999)
    // - Performance hints (6000-6999)
    // - Security warnings (7000-7999)
}

impl From<DiagnosticCode> for u32 {
    fn from(code: DiagnosticCode) -> Self {
        match code {
            DiagnosticCode::LexicalError => 1,
            DiagnosticCode::SyntaxError => 2,
            DiagnosticCode::UnexpectedToken => 3,
            DiagnosticCode::UnexpectedEndOfFile => 4,
            DiagnosticCode::InvalidCharacter => 5,
            DiagnosticCode::UndeclaredVariable => 1001,
            DiagnosticCode::UnusedVariable => 1002,
            DiagnosticCode::DuplicateDefinition => 1003,
            DiagnosticCode::UseBeforeDefinition => 1004,
            DiagnosticCode::TypeMismatch => 2001,
            DiagnosticCode::InvalidFieldAccess => 2002,
            DiagnosticCode::InvalidIndexAccess => 2003,
            DiagnosticCode::InvalidStructLiteral => 2004,
            DiagnosticCode::InvalidFunctionCall => 2005,
            DiagnosticCode::InvalidAssignment => 2006,
            DiagnosticCode::InvalidReturnType => 2007,
            DiagnosticCode::InvalidTypeDefinition => 2008,
            DiagnosticCode::UnreachableCode => 3001,
            DiagnosticCode::MissingReturn => 3002,
        }
    }
}

impl Diagnostic {
    /// Create an error diagnostic
    /// Make const once spanned is given as input
    pub fn error(code: DiagnosticCode, message: String) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            code,
            message,
            span: SimpleSpan::from(0..0),
            related_spans: Vec::new(),
        }
    }

    /// Create a warning diagnostic
    pub fn warning(code: DiagnosticCode, message: String) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            code,
            message,
            span: SimpleSpan::from(0..0),
            related_spans: Vec::new(),
        }
    }

    /// Create an info diagnostic
    pub fn info(code: DiagnosticCode, message: String) -> Self {
        Self {
            severity: DiagnosticSeverity::Info,
            code,
            message,
            span: SimpleSpan::from(0..0),
            related_spans: Vec::new(),
        }
    }

    /// Add location information to this diagnostic
    pub const fn with_location(mut self, span: SimpleSpan<usize>) -> Self {
        self.span = span;
        self
    }

    /// Add a related span with context message
    pub fn with_related_span(mut self, span: SimpleSpan<usize>, message: String) -> Self {
        self.related_spans.push((span, message));
        self
    }

    /// Convenience method for undeclared variable error
    pub fn undeclared_variable(name: &str, span: SimpleSpan<usize>) -> Self {
        Self::error(
            DiagnosticCode::UndeclaredVariable,
            format!("Undeclared variable '{name}'"),
        )
        .with_location(span)
    }

    /// Convenience method for unused variable warning
    pub fn unused_variable(name: &str, span: SimpleSpan<usize>) -> Self {
        Self::warning(
            DiagnosticCode::UnusedVariable,
            format!("Unused variable '{name}'"),
        )
        .with_location(span)
    }

    /// Convenience method for duplicate definition error
    pub fn duplicate_definition(name: &str, span: SimpleSpan<usize>) -> Self {
        Self::error(
            DiagnosticCode::DuplicateDefinition,
            format!("Duplicate definition of '{name}'"),
        )
        .with_location(span)
    }

    /// Convenience method for use before definition error
    pub fn use_before_definition(name: &str, span: SimpleSpan<usize>) -> Self {
        Self::error(
            DiagnosticCode::UseBeforeDefinition,
            format!("Variable '{name}' used before definition"),
        )
        .with_location(span)
    }

    /// Convenience method for unreachable code warning
    pub fn unreachable_code(statement_type: &str, span: SimpleSpan<usize>) -> Self {
        Self::warning(
            DiagnosticCode::UnreachableCode,
            format!("Unreachable {statement_type}"),
        )
        .with_location(span)
    }

    /// Convenience method for missing return warning
    pub fn missing_return(function_name: &str, span: SimpleSpan<usize>) -> Self {
        Self::error(
            DiagnosticCode::MissingReturn,
            format!("Function '{function_name}' doesn't return on all paths"),
        )
        .with_location(span)
    }

    /// Convenience method for lexical errors
    pub fn lexical_error(message: String, span: SimpleSpan<usize>) -> Self {
        Self::error(DiagnosticCode::LexicalError, message).with_location(span)
    }

    /// Convenience method for syntax errors
    pub fn syntax_error(message: String, span: SimpleSpan<usize>) -> Self {
        Self::error(DiagnosticCode::SyntaxError, message).with_location(span)
    }

    /// Convenience method for unexpected token errors
    pub fn unexpected_token(expected: &str, found: &str, span: SimpleSpan<usize>) -> Self {
        Self::error(
            DiagnosticCode::UnexpectedToken,
            format!("Expected {expected}, found {found}"),
        )
        .with_location(span)
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.severity, self.message)?;
        write!(f, " (at {}:{})", self.span.start, self.span.end)?;
        for (span, message) in &self.related_spans {
            write!(f, "\n  note: {} (at {}:{})", message, span.start, span.end)?;
        }
        Ok(())
    }
}

/// Collection of diagnostics from semantic analysis
#[derive(Debug, Default, PartialEq, Eq)]
pub struct DiagnosticCollection {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticCollection {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a diagnostic to the collection
    pub fn add(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Add multiple diagnostics
    pub fn extend(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        self.diagnostics.extend(diagnostics);
    }

    /// Get all diagnostics, sorted by severity and message
    pub fn all(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Get only error diagnostics
    pub fn errors(&self) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .collect()
    }

    /// Get only warning diagnostics
    pub fn warnings(&self) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Warning)
            .collect()
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error)
    }

    /// Get the total number of diagnostics
    pub const fn len(&self) -> usize {
        self.diagnostics.len()
    }

    /// Check if the collection is empty
    pub const fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Sort diagnostics by severity (errors first) and then by message
    pub fn sort(&mut self) {
        self.diagnostics
            .sort_by(|a, b| a.severity.cmp(&b.severity).then(a.message.cmp(&b.message)));
    }

    /// Print all diagnostics to stdout
    pub fn print(&self) {
        for diagnostic in &self.diagnostics {
            println!("{diagnostic}");
        }
    }

    /// Get summary statistics
    pub fn summary(&self) -> String {
        let errors = self.errors().len();
        let warnings = self.warnings().len();
        let total = self.diagnostics.len();

        if total == 0 {
            "No issues found".to_string()
        } else {
            format!("{errors} errors, {warnings} warnings")
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Diagnostic> {
        self.diagnostics.iter()
    }
}

impl From<Vec<Diagnostic>> for DiagnosticCollection {
    fn from(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }
}

impl IntoIterator for DiagnosticCollection {
    type Item = Diagnostic;
    type IntoIter = std::vec::IntoIter<Diagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.diagnostics.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_creation() {
        let span = SimpleSpan::from(10..20);
        let diag = Diagnostic::undeclared_variable("test_var", span);
        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert_eq!(diag.code, DiagnosticCode::UndeclaredVariable);
        assert!(diag.message.contains("test_var"));
        assert_eq!(diag.span, span);
    }

    #[test]
    fn test_diagnostic_collection() {
        let mut collection = DiagnosticCollection::new();

        let span1 = SimpleSpan::from(0..5);
        let span2 = SimpleSpan::from(10..15);
        collection.add(Diagnostic::undeclared_variable("var1", span1));
        collection.add(Diagnostic::unused_variable("var2", span2));

        assert_eq!(collection.len(), 2);
        assert_eq!(collection.errors().len(), 1);
        assert_eq!(collection.warnings().len(), 1);
        assert!(collection.has_errors());
    }

    #[test]
    fn test_diagnostic_display() {
        let span = SimpleSpan::from(5..10);
        let diag = Diagnostic::undeclared_variable("test", span);
        let display = format!("{diag}");
        assert!(display.contains("error"));
        assert!(display.contains("Undeclared variable"));
        assert!(display.contains("test"));
        assert!(display.contains("5:10"));
    }
}
