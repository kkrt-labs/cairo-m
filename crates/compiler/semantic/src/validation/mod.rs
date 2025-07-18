//! # Semantic Validation Framework
//!
//! This module implements validation rules for Cairo-M semantic analysis.
//! It provides a diagnostic system and validator trait pattern for extensible
//! semantic checking.

pub mod literal_validator;
pub mod scope_check;
pub mod type_validator;
pub mod validator;

// TODO: Implement these validators once type system is available
pub mod control_flow_validator;

pub use control_flow_validator::ControlFlowValidator;
pub use literal_validator::LiteralValidator;
pub use scope_check::ScopeValidator;
pub use type_validator::TypeValidator;
pub use validator::Validator;
