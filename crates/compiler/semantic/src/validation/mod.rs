//! # Semantic Validation Framework
//!
//! This module implements validation rules for Cairo-M semantic analysis.
//! It provides a diagnostic system and validator trait pattern for extensible
//! semantic checking.

pub mod control_flow_validator;
pub mod literal_validator;
pub mod scope_check;
pub mod shared;
pub mod structural_validator;
pub mod type_validator;
pub mod validator;

pub use control_flow_validator::ControlFlowValidator;
pub use literal_validator::LiteralValidator;
pub use scope_check::ScopeValidator;
pub use structural_validator::StructuralValidator;
pub use type_validator::TypeValidator;
pub use validator::Validator;
