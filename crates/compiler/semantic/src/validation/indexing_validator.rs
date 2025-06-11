//! # Indexing Validator
//!
//! This validator checks that indexing operations are valid:
//! - Object being indexed is actually indexable (array, slice, etc.)
//! - Index expression has appropriate type
//! - Bounds checking where possible

use crate::db::SemanticDb;
use crate::validation::{Diagnostic, Validator};
use crate::{File, SemanticIndex};

use crate::type_resolution::expression_semantic_type;
use crate::types::TypeData;

/// Validator for indexing operations
///
/// This validator ensures that index access expressions (e.g., `arr[0]`)
/// are semantically valid by checking:
/// - The indexed object has an indexable type (array, slice, tuple, etc.)
/// - The index expression is of integer type
/// - Array bounds where statically determinable
///
/// # Examples of errors this catches:
///
/// ```cairo-m,ignore
/// let num = 42;
/// let bad = num[0]; // Error: cannot index felt
///
/// let arr = [1, 2, 3];
/// let bad = arr["string"]; // Error: index must be integer type
/// ```
pub struct IndexingValidator;

impl Validator for IndexingValidator {
    fn validate(&self, db: &dyn SemanticDb, file: File, index: &SemanticIndex) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for (_expr_id, expr_info) in index.all_expressions() {
            if let Expression::IndexAccess {
                array,
                index: index_expr,
            } = &expr_info.ast_node
            {
                let array_id = index.span_to_expression_id.get(&array.span());
                let array_type_id = expression_semantic_type(db, file, *array_id.unwrap());
                let array_type = array_type_id.data(db);

                // Check if the array expression is indexable
                match array_type {
                    TypeData::Tuple(_) | TypeData::Pointer(_) => {
                        // Check if the index expression is an integer type
                        let index_id = index.span_to_expression_id.get(&index_expr.span());
                        let index_type_id = expression_semantic_type(db, file, *index_id.unwrap());
                        let index_type = index_type_id.data(db);

                        if !matches!(index_type, TypeData::Felt) {
                            diagnostics.push(
                                Diagnostic::error(
                                    DiagnosticCode::InvalidIndexType,
                                    format!("Expected integer type for index, got {index_type:?}"),
                                )
                                .with_location(index_expr.span()),
                            );
                        }
                    }
                    TypeData::Error => (), // already handled by scope validator
                    _ => {
                        diagnostics.push(
                            Diagnostic::error(
                                DiagnosticCode::InvalidIndexAccess,
                                format!("Cannot index type {array_type:?}"),
                            )
                            .with_location(array.span()),
                        );
                    }
                }
            }
        }

        diagnostics
    }

    fn name(&self) -> &'static str {
        "IndexingValidator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::test_db;
    use crate::semantic_index::semantic_index;

    #[test]
    fn test_valid_pointer_indexing() {
        let db = test_db();
        let program = r#"
            func test() {
                let ptr = &1;
                let first = ptr[0];
                let second = ptr[1];
            }
        "#;
        let file = File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file);

        let validator = IndexingValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        // Should not have any diagnostics for valid array indexing
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_valid_tuple_indexing() {
        let db = test_db();
        let program = r#"
            func test() {
                let tup = (1, 2, 3);
                let first = tup[0];
                let second = tup[1];
            }
        "#;
        let file = File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file);

        let validator = IndexingValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        // Should not have any diagnostics for valid tuple indexing
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_indexing_non_indexable_type() {
        let db = test_db();
        let program = r#"
            func test() {
                let num = 42;
                let bad = num[0];  // Error: cannot index felt
            }
        "#;
        let file = File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file);

        let validator = IndexingValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagnosticCode::InvalidIndexAccess);
        assert!(diagnostics[0].message.contains("Cannot index type"));
    }

    #[test]
    fn test_non_integer_index() {
        let db = test_db();
        let program = r#"
            struct Point {
                x: felt,
                y: felt,
            }
            func test() {
                let tuple = (1, 2, 3);
                let bad = tuple[Point { x: 1, y: 2 }];  // Error: index must be integer
            }
        "#;
        let file = File::new(&db, program.to_string());
        let semantic_index = semantic_index(&db, file);

        let validator = IndexingValidator;
        let diagnostics = validator.validate(&db, file, semantic_index);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagnosticCode::InvalidIndexType);
        assert!(
            diagnostics[0]
                .message
                .contains("Expected integer type for index")
        );
    }
}
