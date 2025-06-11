//! # Struct Field Access Validator
//!
//! This validator checks that field access operations are valid:
//! - Field exists on the struct type
//! - Object is actually a struct type (not primitive)
//! - Field access is used correctly
//! - Struct literals have all required fields
//! - Struct literals don't have unknown fields

use crate::db::SemanticDb;
use crate::type_resolution::expression_semantic_type;
use crate::types::TypeData;
use crate::validation::{Diagnostic, Validator};
use crate::DiagnosticCode;
use crate::{File, SemanticIndex};
use cairo_m_compiler_parser::parser::Expression;
use std::collections::HashSet;

/// Validator for struct field access operations and struct literals
///
/// This validator ensures that member access expressions (e.g., `obj.field`)
/// are semantically valid by checking:
/// - The object has a struct type that contains the accessed field
/// - The field name exists on the struct definition
/// - The object is not a primitive type (felt, etc.)
///
/// It also validates struct literal expressions (e.g., `Point { x: 1, y: 2 }`)
/// by checking:
/// - All required fields are provided
/// - No unknown fields are specified
/// - The struct type exists and is valid
///
/// # Examples of errors this catches:
///
/// ```cairo-m,ignore
/// let p = Point { x: 1, y: 2 };
/// let bad = p.z; // Error: field 'z' doesn't exist on Point
///
/// let p_incomplete = Point { x: 1 }; // Error: missing field 'y'
/// let p_extra = Point { x: 1, y: 2, z: 3 }; // Error: unknown field 'z'
///
/// let num = 42;
/// let bad = num.field; // Error: felt has no fields
/// ```
pub struct StructFieldValidator;

impl Validator for StructFieldValidator {
    fn validate(&self, db: &dyn SemanticDb, file: File, index: &SemanticIndex) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for (_expr_id, expr_info) in index.all_expressions() {
            match &expr_info.ast_node {
                Expression::MemberAccess { object, field } => {
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
                                    .with_location(field.span()),
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
                Expression::StructLiteral { name, fields } => {
                    // First, try to resolve the struct type
                    if let Some((def_idx, _)) =
                        index.resolve_name_to_definition(name.value(), expr_info.scope_id)
                    {
                        use crate::semantic_index::DefinitionId;
                        use crate::type_resolution::definition_semantic_type;

                        let def_id = DefinitionId::new(db, file, def_idx);
                        let def_type = definition_semantic_type(db, def_id);

                        if let TypeData::Struct(struct_type) = def_type.data(db) {
                            let struct_fields = struct_type.fields(db);
                            let provided_fields: HashSet<String> = fields
                                .iter()
                                .map(|(field_name, _)| field_name.value().clone())
                                .collect();

                            // Check for missing fields
                            for (field_name, _) in &struct_fields {
                                if !provided_fields.contains(field_name) {
                                    diagnostics.push(
                                        Diagnostic::error(
                                            DiagnosticCode::InvalidStructLiteral,
                                            format!(
                                                "Missing field '{}' in struct literal for {}",
                                                field_name,
                                                struct_type.name(db)
                                            ),
                                        )
                                        .with_location(name.span()),
                                    );
                                }
                            }

                            // Check for unknown fields
                            for (field_name, _) in fields {
                                if !struct_fields
                                    .iter()
                                    .any(|(name, _)| name == field_name.value())
                                {
                                    diagnostics.push(
                                        Diagnostic::error(
                                            DiagnosticCode::InvalidFieldAccess,
                                            format!(
                                                "Field {} does not exist in struct {}",
                                                field_name.value(),
                                                struct_type.name(db)
                                            ),
                                        )
                                        .with_location(field_name.span()),
                                    );
                                }
                            }
                        } else {
                            diagnostics.push(
                                Diagnostic::error(
                                    DiagnosticCode::InvalidStructLiteral,
                                    format!("Expected struct type, found {}", name.value()),
                                )
                                .with_location(name.span()),
                            );
                        }
                    } else {
                        // This will be caught by the scope validator as undeclared variable
                        // so we don't need to emit an error here
                    }
                }
                _ => {}
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

    #[test]
    fn test_incomplete_struct_literal() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            func test() {
                let p = Point { x: 1 };  // Error: missing field 'y'
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
            .contains("Missing field 'y' in struct literal"));
    }

    #[test]
    fn test_struct_literal_unknown_field() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            func test() {
                let p = Point { x: 1, y: 2, z: 3 };  // Error: unknown field 'z'
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
            .contains("Field z does not exist in struct Point"));
    }

    #[test]
    fn test_struct_literal_multiple_missing_fields() {
        let db = test_db();
        let program = r#"
            struct Rectangle { x: felt, y: felt, width: felt, height: felt }
            func test() {
                let r = Rectangle { x: 1 };  // Error: missing fields 'y', 'width', 'height'
            }
        "#;
        let file = File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file)
            .as_ref()
            .expect("Got unexpected parse errors");

        let validator = StructFieldValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        // Should have 3 errors for the 3 missing fields
        assert_eq!(diagnostics.len(), 3);
        for diagnostic in &diagnostics {
            assert_eq!(diagnostic.code, DiagnosticCode::InvalidStructLiteral);
            assert!(diagnostic.message.contains("Missing field"));
        }

        // Check that all expected missing fields are reported
        let missing_fields: Vec<&str> = diagnostics
            .iter()
            .filter_map(|d| {
                if d.message.contains("Missing field 'y'") {
                    Some("y")
                } else if d.message.contains("Missing field 'width'") {
                    Some("width")
                } else if d.message.contains("Missing field 'height'") {
                    Some("height")
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(missing_fields.len(), 3);
        assert!(missing_fields.contains(&"y"));
        assert!(missing_fields.contains(&"width"));
        assert!(missing_fields.contains(&"height"));
    }

    #[test]
    fn test_empty_struct_literal() {
        let db = test_db();
        let program = r#"
            struct Empty { }
            func test() {
                let e = Empty { };  // Valid: no fields required
            }
        "#;
        let file = File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file)
            .as_ref()
            .expect("Got unexpected parse errors");

        let validator = StructFieldValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        // Should not have any diagnostics for valid empty struct
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_complete_struct_literal() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            func test() {
                let p = Point { x: 1, y: 2 };  // Valid: all fields provided
            }
        "#;
        let file = File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file)
            .as_ref()
            .expect("Got unexpected parse errors");

        let validator = StructFieldValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        // Should not have any diagnostics for valid complete struct
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_nested_struct_literal_validation() {
        let db = test_db();
        let program = r#"
            struct Point { x: felt, y: felt }
            struct Line { start: Point, end: Point }
            func test() {
                let line = Line {
                    start: Point { x: 1 },    // Error: missing 'y' in Point
                    end: Point { x: 3, y: 4, z: 5 }  // Error: unknown field 'z' in Point
                };
            }
        "#;
        let file = File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file)
            .as_ref()
            .expect("Got unexpected parse errors");

        let validator = StructFieldValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        assert_eq!(diagnostics.len(), 2);

        // Check for missing field error
        let missing_field_error = diagnostics
            .iter()
            .find(|d| d.message.contains("Missing field 'y'"));
        assert!(missing_field_error.is_some());
        assert_eq!(
            missing_field_error.unwrap().code,
            DiagnosticCode::InvalidStructLiteral
        );

        // Check for unknown field error
        let unknown_field_error = diagnostics
            .iter()
            .find(|d| d.message.contains("Field z does not exist"));
        assert!(unknown_field_error.is_some());
        assert_eq!(
            unknown_field_error.unwrap().code,
            DiagnosticCode::InvalidFieldAccess
        );
    }
}
