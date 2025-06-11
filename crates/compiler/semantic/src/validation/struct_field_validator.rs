//! # Struct Field Access Validator
//!
//! This validator checks that field access operations are valid:
//! - Field exists on the struct type
//! - Object is actually a struct type (not primitive)
//! - Field access is used correctly

use crate::db::SemanticDb;
use crate::type_resolution::expression_semantic_type;
use crate::types::TypeData;
use crate::validation::{Diagnostic, Validator};
use crate::DiagnosticCode;
use crate::{File, SemanticIndex};
use cairo_m_compiler_parser::parser::Expression;

/// Validator for struct field access operations
///
/// This validator ensures that member access expressions (e.g., `obj.field`)
/// are semantically valid by checking:
/// - The object has a struct type that contains the accessed field
/// - The field name exists on the struct definition
/// - The object is not a primitive type (felt, etc.)
///
/// # Examples of errors this catches:
///
/// ```cairo-m,ignore
/// let p = Point { x: 1, y: 2 };
/// let bad = p.z; // Error: field 'z' doesn't exist on Point
///
/// let num = 42;
/// let bad = num.field; // Error: felt has no fields
/// ```
pub struct StructFieldValidator;

impl Validator for StructFieldValidator {
    fn validate(&self, db: &dyn SemanticDb, file: File, index: &SemanticIndex) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for (_expr_id, expr_info) in index.all_expressions() {
            if let Expression::MemberAccess { object, field } = &expr_info.ast_node {
                let object_id = index.span_to_expression_id.get(&object.span());
                let object_type_id = expression_semantic_type(db, file, *object_id.unwrap());
                let object_type = object_type_id.data(db);
                match object_type {
                    TypeData::Struct(struct_type) => {
                        let fields = struct_type.fields(db);
                        if !fields.iter().any(|(name, _)| name == field.value()) {
                            diagnostics.push(
                                Diagnostic::error(
                                    DiagnosticCode::InvalidFieldAccess,
                                    format!(
                                        "Field {} does not exist in struct {}",
                                        field.value(),
                                        struct_type.name(db)
                                    ),
                                )
                                .with_location(object.span()),
                            );
                        }
                    }
                    TypeData::Error => (), // already handled by scope validator
                    _ => {
                        diagnostics.push(
                            Diagnostic::error(
                                DiagnosticCode::InvalidStructLiteral,
                                format!("Expected struct type, got {object_type:?}"),
                            )
                            .with_location(object.span()),
                        );
                    }
                }
            }
        }

        diagnostics
    }

    fn name(&self) -> &'static str {
        "StructFieldValidator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::test_db;
    use crate::semantic_index::semantic_index;

    #[test]
    fn test_valid_field_access() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            func test() {
                let p = Point { x: 1, y: 2 };
                let x = p.x;
                let y = p.y;
            }
        "#;
        let file = File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file)
            .as_ref()
            .expect("Got unexpected parse errors");

        let validator = StructFieldValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        // Should not have any diagnostics for valid field access
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_invalid_field_access() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            func test() {
                let p = Point { x: 1, y: 2 };
                let z = p.z;  // Error: field 'z' doesn't exist
            }
        "#;
        let file = File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file)
            .as_ref()
            .expect("Got unexpected parse errors");

        let validator = StructFieldValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagnosticCode::InvalidFieldAccess);
        assert!(diagnostics[0].message.contains("Field z does not exist"));
    }

    #[test]
    fn test_field_access_on_primitive() {
        let db = test_db();
        let program = r#"
            func test() {
                let num = 42;
                let bad = num.field;  // Error: felt has no fields
            }
        "#;
        let file = File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file)
            .as_ref()
            .expect("Got unexpected parse errors");

        let validator = StructFieldValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagnosticCode::InvalidStructLiteral);
        assert!(diagnostics[0]
            .message
            .contains("Expected struct type, got Felt"));
    }

    #[test]
    fn test_nested_field_access() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            struct Line { start: Point, end: Point }
            func test() {
                let line = Line {
                    start: Point { x: 1, y: 2 },
                    end: Point { x: 3, y: 4 }
                };
                let x = line.start.x;  // Valid nested access
                let bad = line.middle; // Error: field doesn't exist
            }
        "#;
        let file = File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file)
            .as_ref()
            .expect("Got unexpected parse errors");

        let validator = StructFieldValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagnosticCode::InvalidFieldAccess);
        assert!(diagnostics[0]
            .message
            .contains("Field middle does not exist"));
    }
}
