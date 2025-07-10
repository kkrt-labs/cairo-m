//! Real integration tests that actually test type resolution
//!
//! These tests verify that the type system actually works end-to-end

use std::collections::HashMap;

use cairo_m_compiler_parser::parser::TypeExpr as AstTypeExpr;

use crate::db::Project;
use crate::db::tests::test_db;
use crate::semantic_index::DefinitionId;
use crate::type_resolution::{
    definition_semantic_type, expression_semantic_type, function_semantic_signature,
    resolve_ast_type, struct_semantic_data,
};
use crate::types::{TypeData, TypeId};
use crate::{File, FileScopeId, SemanticDb, SemanticIndex, project_semantic_index};

fn single_file_project(db: &dyn SemanticDb, file: File) -> Project {
    let mut modules = HashMap::new();
    modules.insert("main".to_string(), file);
    Project::new(db, modules, "main".to_string())
}

fn get_root_scope(db: &dyn SemanticDb, project: Project) -> FileScopeId {
    let semantic_index = project_semantic_index(db, project).unwrap();
    semantic_index
        .modules()
        .get("main")
        .unwrap()
        .root_scope()
        .unwrap()
}

fn get_main_semantic_index(db: &dyn SemanticDb, project: Project) -> SemanticIndex {
    let semantic_index = project_semantic_index(db, project).unwrap();
    semantic_index.modules().get("main").unwrap().clone()
}

#[test]
fn test_resolve_primitive_types() {
    let db = test_db();
    let file = File::new(&db, "".to_string(), "test.cm".to_string());
    let project = single_file_project(&db, file);
    let root_scope = get_root_scope(&db, project);

    let felt_type = resolve_ast_type(
        &db,
        project,
        file,
        AstTypeExpr::Named("felt".to_string()),
        root_scope,
    );
    assert!(matches!(felt_type.data(&db), TypeData::Felt));

    let pointer_felt_type = resolve_ast_type(
        &db,
        project,
        file,
        AstTypeExpr::Pointer(Box::new(AstTypeExpr::Named("felt".to_string()))),
        root_scope,
    );
    assert!(
        matches!(pointer_felt_type.data(&db), TypeData::Pointer(t) if matches!(t.data(&db), TypeData::Felt))
    );
}

#[test]
fn test_struct_type_resolution() {
    let db = test_db();
    let program = r#"
        struct Point {
            x: felt,
            y: felt,
        }
    "#;
    let file = File::new(&db, program.to_string(), "test.cm".to_string());
    let project = single_file_project(&db, file);
    let semantic_index = get_main_semantic_index(&db, project);
    let root_scope = semantic_index.root_scope().unwrap();

    // 1. Resolve `Point` as a type name.
    let point_type_id = resolve_ast_type(
        &db,
        project,
        file,
        AstTypeExpr::Named("Point".to_string()),
        root_scope,
    );
    let point_type_data = point_type_id.data(&db);

    // 2. Assert it resolved to a Struct type.
    let struct_id = match point_type_data {
        TypeData::Struct(id) => {
            assert_eq!(id.name(&db), "Point");
            id
        }
        other => panic!("Expected Point to resolve to a struct, got {other:?}"),
    };

    // 3. Get the struct's semantic data directly and compare.
    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("Point", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);
    let semantic_data = struct_semantic_data(&db, project, def_id).unwrap();

    assert_eq!(struct_id, semantic_data);
    assert_eq!(semantic_data.name(&db), "Point");

    // 4. Check the fields.
    let fields = semantic_data.fields(&db);
    let felt_type = TypeId::new(&db, TypeData::Felt);
    let expected_fields = vec![("x".to_string(), felt_type), ("y".to_string(), felt_type)];
    assert_eq!(fields, expected_fields);
}

#[test]
fn test_function_signature_resolution() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }
        func get_point(x: felt) -> Point {
            return Point { x: x, y: 0 };
        }
    "#;
    let file = File::new(&db, program.to_string(), "test.cm".to_string());
    let project = single_file_project(&db, file);
    let semantic_index = get_main_semantic_index(&db, project);
    let root_scope = semantic_index.root_scope().unwrap();

    // 1. Get the function definition.
    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("get_point", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);

    // 2. Resolve the function's signature.
    let signature = function_semantic_signature(&db, project, def_id).unwrap();
    let params = signature.params(&db);
    let return_type = signature.return_type(&db);

    // 3. Assert parameter types.
    let felt_type = TypeId::new(&db, TypeData::Felt);
    let expected_params = vec![("x".to_string(), felt_type)];
    assert_eq!(params, expected_params);

    // 4. Assert return type.
    let point_type_id = resolve_ast_type(
        &db,
        project,
        file,
        AstTypeExpr::Named("Point".to_string()),
        root_scope,
    );
    assert_eq!(return_type, point_type_id);
    assert!(matches!(return_type.data(&db), TypeData::Struct(_)));

    // 5. Check the full function type from its definition.
    let func_type = definition_semantic_type(&db, project, def_id);
    match func_type.data(&db) {
        TypeData::Function(sig_id) => assert_eq!(sig_id, signature),
        other => panic!("Expected function type, got {other:?}"),
    }
}

#[test]
fn test_parameter_type_resolution() {
    let db = test_db();
    let program = r#"
        struct Vector { x: felt, y: felt }
        func magnitude(v: Vector) -> felt {
            return 0;
        }
    "#;
    let file = File::new(&db, program.to_string(), "test.cm".to_string());
    let project = single_file_project(&db, file);
    let semantic_index = get_main_semantic_index(&db, project);
    let root_scope = semantic_index.root_scope().unwrap();
    let func_scope = semantic_index
        .child_scopes(root_scope)
        .find(|s| semantic_index.scope(*s).unwrap().kind == crate::place::ScopeKind::Function)
        .unwrap();

    // 1. Find the parameter definition `v`.
    let (param_def_idx, _) = semantic_index
        .resolve_name_to_definition("v", func_scope)
        .unwrap();
    let param_def_id = DefinitionId::new(&db, file, param_def_idx);

    // 2. Get its semantic type.
    let param_type = definition_semantic_type(&db, project, param_def_id);

    // 3. Assert it's a struct type `Vector`.
    match param_type.data(&db) {
        TypeData::Struct(struct_id) => {
            assert_eq!(struct_id.name(&db), "Vector");
        }
        other => panic!("Expected parameter to be a struct type, got {other:?}"),
    }
}

#[test]
fn test_expression_type_inference() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }
        func test(p: Point) -> felt {
            let a = 42;
            let b = a + 1;
            let c = p.x;
            return c;
        }
    "#;
    let file = File::new(&db, program.to_string(), "test.cm".to_string());
    let project = single_file_project(&db, file);
    let semantic_index = get_main_semantic_index(&db, project);

    // Helper to find an expression by matching against tracked expressions
    let find_expr_id = |target_text: &str| {
        for (span, expr_id) in &semantic_index.span_to_expression_id {
            let source_text = &program[span.start..span.end];
            if source_text == target_text {
                return *expr_id;
            }
        }
        panic!(
            "Expression '{}' not found in tracked expressions. Available: {:?}",
            target_text,
            semantic_index
                .span_to_expression_id
                .keys()
                .map(|span| &program[span.into_range()])
                .collect::<Vec<_>>()
        );
    };

    let felt_type = TypeId::new(&db, TypeData::Felt);

    // Test literal
    let expr_id = find_expr_id("42");
    let expr_type = expression_semantic_type(&db, project, file, expr_id);
    assert_eq!(expr_type, felt_type);

    // Test identifier `a` (inferred from literal)
    let a_expr_id = find_expr_id("a");
    let a_expr_type = expression_semantic_type(&db, project, file, a_expr_id);
    assert_eq!(a_expr_type, felt_type);

    // Test binary operation
    let expr_id = find_expr_id("a + 1");
    let expr_type = expression_semantic_type(&db, project, file, expr_id);
    assert_eq!(expr_type, felt_type);

    // Test member access
    let expr_id = find_expr_id("p.x");
    let expr_type = expression_semantic_type(&db, project, file, expr_id);
    assert_eq!(expr_type, felt_type);

    // Test identifier `c` (inferred from member access)
    // Find the identifier 'c' in the return statement
    let c_expr_id = find_expr_id("c");
    let c_expr_type = expression_semantic_type(&db, project, file, c_expr_id);
    assert_eq!(c_expr_type, felt_type);
}

#[test]
fn test_let_variable_type_inference() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }
        func test() {
            let a = 42;
            let b = 13;
            let p = Point { x: 1, y: 2 };
        }
    "#;
    let file = File::new(&db, program.to_string(), "test.cm".to_string());
    let project = single_file_project(&db, file);
    let semantic_index = get_main_semantic_index(&db, project);
    let func_scope = semantic_index
        .scopes()
        .find(|(_, scope)| scope.kind == crate::place::ScopeKind::Function)
        .map(|(scope_id, _)| scope_id)
        .unwrap();

    // Helper function to get variable type
    let get_var_type = |var_name: &str| {
        let (def_idx, _) = semantic_index
            .resolve_name_to_definition(var_name, func_scope)
            .unwrap_or_else(|| panic!("Variable '{var_name}' not found"));
        let def_id = DefinitionId::new(&db, file, def_idx);
        definition_semantic_type(&db, project, def_id)
    };

    let felt_type = TypeId::new(&db, TypeData::Felt);

    // Test: `let a = 42` should infer `felt`
    let a_type = get_var_type("a");
    assert_eq!(a_type, felt_type, "Variable 'a' should be inferred as felt");

    // Test: `let b = 13` should also infer `felt`
    let b_type = get_var_type("b");
    assert_eq!(
        b_type, felt_type,
        "Variable 'b' should be inferred as felt, not affected by other variables"
    );

    // Test: `let p = Point { x: 1, y: 2 }` should infer struct type
    let p_type = get_var_type("p");
    match p_type.data(&db) {
        TypeData::Struct(struct_id) => {
            assert_eq!(struct_id.name(&db), "Point");
        }
        other => panic!("Variable 'p' should be inferred as Point struct, got {other:?}"),
    }
}

#[test]
fn test_const_variable_type_inference() {
    let db = test_db();
    let program = r#"
        const MAGIC_NUMBER = 42;
        const PI_APPROX = 314;
    "#;
    let file = File::new(&db, program.to_string(), "test.cm".to_string());
    let project = single_file_project(&db, file);
    let semantic_index = get_main_semantic_index(&db, project);
    let root_scope = semantic_index.root_scope().unwrap();

    // Helper function to get constant type
    let get_const_type = |const_name: &str| {
        let (def_idx, _) = semantic_index
            .resolve_name_to_definition(const_name, root_scope)
            .unwrap_or_else(|| panic!("Constant '{const_name}' not found"));
        let def_id = DefinitionId::new(&db, file, def_idx);
        definition_semantic_type(&db, project, def_id)
    };

    let felt_type = TypeId::new(&db, TypeData::Felt);

    // Test: Both constants should infer felt type correctly
    let magic_type = get_const_type("MAGIC_NUMBER");
    assert_eq!(
        magic_type, felt_type,
        "MAGIC_NUMBER should be inferred as felt"
    );

    let pi_type = get_const_type("PI_APPROX");
    assert_eq!(
        pi_type, felt_type,
        "PI_APPROX should be inferred as felt, not affected by other constants"
    );
}

#[test]
fn test_explicit_type_annotations_priority() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }
        func test() {
            let x: felt = 42;
            local y: Point = 13;
            let p: Point = Point { x: 1, y: 2 };
        }
    "#;
    let file = File::new(&db, program.to_string(), "test.cm".to_string());
    let project = single_file_project(&db, file);
    let semantic_index = get_main_semantic_index(&db, project);
    let func_scope = semantic_index
        .scopes()
        .find(|(_, scope)| scope.kind == crate::place::ScopeKind::Function)
        .map(|(scope_id, _)| scope_id)
        .unwrap();

    // Helper function to get variable type
    let get_var_type = |var_name: &str| {
        let (def_idx, _) = semantic_index
            .resolve_name_to_definition(var_name, func_scope)
            .unwrap_or_else(|| panic!("Variable '{var_name}' not found"));
        let def_id = DefinitionId::new(&db, file, def_idx);
        definition_semantic_type(&db, project, def_id)
    };

    let felt_type = TypeId::new(&db, TypeData::Felt);

    // Test: `let x: felt = 42` should use explicit type annotation
    let x_type = get_var_type("x");
    assert_eq!(
        x_type, felt_type,
        "Variable 'x' should use explicit felt annotation"
    );

    // Test: `local y: Point = 13` should use explicit type annotation (even if incorrect)
    let y_type = get_var_type("y");
    match y_type.data(&db) {
        TypeData::Struct(struct_id) => {
            assert_eq!(struct_id.name(&db), "Point");
        }
        other => panic!("Variable 'y' should use explicit Point annotation, got {other:?}"),
    }

    // Test: `let p: Point = Point { x: 1, y: 2 }` should use explicit struct annotation
    let p_type = get_var_type("p");
    match p_type.data(&db) {
        TypeData::Struct(struct_id) => {
            assert_eq!(struct_id.name(&db), "Point");
        }
        other => panic!("Variable 'p' should use explicit Point annotation, got {other:?}"),
    }
}

#[test]
fn test_local_variable_inference_without_annotation() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }
        func test() {
            local x = 42;
            local y = Point { x: 1, y: 2 };
        }
    "#;
    let file = File::new(&db, program.to_string(), "test.cm".to_string());
    let project = single_file_project(&db, file);
    let semantic_index = get_main_semantic_index(&db, project);
    let func_scope = semantic_index
        .scopes()
        .find(|(_, scope)| scope.kind == crate::place::ScopeKind::Function)
        .map(|(scope_id, _)| scope_id)
        .unwrap();

    // Helper function to get variable type
    let get_var_type = |var_name: &str| {
        let (def_idx, _) = semantic_index
            .resolve_name_to_definition(var_name, func_scope)
            .unwrap_or_else(|| panic!("Variable '{var_name}' not found"));
        let def_id = DefinitionId::new(&db, file, def_idx);
        definition_semantic_type(&db, project, def_id)
    };

    let felt_type = TypeId::new(&db, TypeData::Felt);

    // Test: local variables without explicit types should infer from their values
    let x_type = get_var_type("x");
    assert_eq!(
        x_type, felt_type,
        "Local variable 'x' should be inferred as felt"
    );

    let y_type = get_var_type("y");
    match y_type.data(&db) {
        TypeData::Struct(struct_id) => {
            assert_eq!(struct_id.name(&db), "Point");
        }
        other => panic!("Variable 'y' should be inferred Point, got {other:?}"),
    }
}

#[test]
fn test_mixed_variable_scenarios() {
    let db = test_db();
    let program = r#"
        struct Vector { x: felt, y: felt }
        func complex_test() {
            let a = 42;                    // infer from literal
            let b: felt = a + 1;           // explicit annotation, infer from expression
            local c = Vector { x: 1, y: 2 }; // infer from struct literal
            local d: Vector = c;           // explicit annotation, infer from identifier
        }
    "#;
    let file = File::new(&db, program.to_string(), "test.cm".to_string());
    let project = single_file_project(&db, file);
    let semantic_index = get_main_semantic_index(&db, project);
    let func_scope = semantic_index
        .scopes()
        .find(|(_, scope)| scope.kind == crate::place::ScopeKind::Function)
        .map(|(scope_id, _)| scope_id)
        .unwrap();

    // Helper function to get variable type
    let get_var_type = |var_name: &str| {
        let (def_idx, _) = semantic_index
            .resolve_name_to_definition(var_name, func_scope)
            .unwrap_or_else(|| panic!("Variable '{var_name}' not found"));
        let def_id = DefinitionId::new(&db, file, def_idx);
        definition_semantic_type(&db, project, def_id)
    };

    let felt_type = TypeId::new(&db, TypeData::Felt);

    // Test all variables get the correct types
    let a_type = get_var_type("a");
    assert_eq!(a_type, felt_type, "Variable 'a' should be inferred as felt");

    let b_type = get_var_type("b");
    assert_eq!(
        b_type, felt_type,
        "Variable 'b' should use explicit felt annotation"
    );

    let c_type = get_var_type("c");
    match c_type.data(&db) {
        TypeData::Struct(struct_id) => {
            assert_eq!(struct_id.name(&db), "Vector");
        }
        other => panic!("Variable 'c' should be inferred as Vector struct, got {other:?}"),
    }

    let d_type = get_var_type("d");
    match d_type.data(&db) {
        TypeData::Struct(struct_id) => {
            assert_eq!(struct_id.name(&db), "Vector");
        }
        other => panic!("Variable 'd' should use explicit Vector annotation, got {other:?}"),
    }
}

#[test]
fn test_multiple_return_type_signature() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }
        func my_func() -> (felt, Point) {
            return (1, Point { x: 2, y: 3 });
        }
    "#;
    let file = File::new(&db, program.to_string(), "test.cm".to_string());
    let project = single_file_project(&db, file);
    let semantic_index = get_main_semantic_index(&db, project);
    let root_scope = semantic_index.root_scope().unwrap();

    // 1. Get the function definition.
    let (def_idx, _) = semantic_index
        .resolve_name_to_definition("my_func", root_scope)
        .unwrap();
    let def_id = DefinitionId::new(&db, file, def_idx);

    // 2. Resolve the function's signature.
    let signature = function_semantic_signature(&db, project, def_id).unwrap();
    let return_type = signature.return_type(&db);

    // 3. Assert return type is a tuple.
    match return_type.data(&db) {
        TypeData::Tuple(elements) => {
            assert_eq!(elements.len(), 2, "Tuple should have 2 elements");

            // First element should be felt
            let felt_type = TypeId::new(&db, TypeData::Felt);
            assert_eq!(elements[0], felt_type, "First element should be felt");

            // Second element should be Point struct
            match elements[1].data(&db) {
                TypeData::Struct(struct_id) => {
                    assert_eq!(struct_id.name(&db), "Point");
                }
                other => panic!("Second element should be Point struct, got {other:?}"),
            }
        }
        other => panic!("Expected return type to be a tuple, got {other:?}"),
    }

    // 4. Check the full function type from its definition.
    let func_type = definition_semantic_type(&db, project, def_id);
    match func_type.data(&db) {
        TypeData::Function(sig_id) => assert_eq!(sig_id, signature),
        other => panic!("Expected function type, got {other:?}"),
    }
}
