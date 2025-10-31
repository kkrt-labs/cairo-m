//! Tests for pointer types and heap allocation `new` expression

use cairo_m_compiler_semantic::semantic_index::DefinitionId;

use super::*;
use crate::{crate_from_program, get_main_semantic_index};

#[test]
fn resolve_pointer_types() {
    let db = test_db();
    let code = r#"
        struct Point { x: felt, y: felt }
        fn f(p: felt*, q: u32*, r: Point*) { }
    "#;
    let crate_id = crate_from_program(&db, code);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let index = get_main_semantic_index(&db, crate_id);

    // Find function definition and check param types
    for (def_idx, def) in index.all_definitions() {
        if def.name == "f" {
            let def_id = DefinitionId::new(&db, file, def_idx);
            if let Some(sig) =
                cairo_m_compiler_semantic::type_resolution::function_semantic_signature(
                    &db, crate_id, def_id,
                )
                .as_ref()
            {
                let params = sig.params(&db);
                assert_eq!(params.len(), 3);
                match params[0].1.data(&db) {
                    TypeData::Pointer { .. } => {}
                    other => panic!("expected pointer, got {other:?}"),
                }
                match params[1].1.data(&db) {
                    TypeData::Pointer { .. } => {}
                    other => panic!("expected pointer, got {other:?}"),
                }
                match params[2].1.data(&db) {
                    TypeData::Pointer { .. } => {}
                    other => panic!("expected pointer, got {other:?}"),
                }
            }
        }
    }
}

#[test]
fn new_expression_and_index_typing() {
    let db = test_db();
    let program = r#"
        struct Point { x: felt, y: felt }
        fn test(n: felt, i: felt) {
            let pf: felt* = new felt[10];
            let uf: u32* = new u32[n];
            let ps: Point* = new Point[2];

            let a = pf[0];        // felt
            let b = uf[i];        // u32
            let c = ps[1].x;      // felt
        }
    "#;
    let crate_id = crate_from_program(&db, program);
    let file = *crate_id.modules(&db).values().next().unwrap();
    let index = get_main_semantic_index(&db, crate_id);

    // Find function scope
    let root_scope = index.root_scope().unwrap();
    let func_scope = index
        .child_scopes(root_scope)
        .find(|s| {
            index.scope(*s).unwrap().kind == cairo_m_compiler_semantic::place::ScopeKind::Function
        })
        .unwrap();

    // pf/uf/ps should be pointer types
    for name in ["pf", "uf", "ps"] {
        let def_idx = index
            .latest_definition_index_by_name(func_scope, name)
            .unwrap();
        let def_id = DefinitionId::new(&db, file, def_idx);
        let ty = cairo_m_compiler_semantic::type_resolution::definition_semantic_type(
            &db, crate_id, def_id,
        );
        assert!(
            matches!(ty.data(&db), TypeData::Pointer { .. }),
            "{} should be pointer type",
            name
        );
    }

    // b (let b = uf[i]) must be u32
    let b_def_idx = index
        .latest_definition_index_by_name(func_scope, "b")
        .unwrap();
    let b_def_id = DefinitionId::new(&db, file, b_def_idx);
    let b_ty = cairo_m_compiler_semantic::type_resolution::definition_semantic_type(
        &db, crate_id, b_def_id,
    );
    assert!(matches!(b_ty.data(&db), TypeData::U32));

    // c (let c = ps[1].x) must be felt
    let c_def_idx = index
        .latest_definition_index_by_name(func_scope, "c")
        .unwrap();
    let c_def_id = DefinitionId::new(&db, file, c_def_idx);
    let c_ty = cairo_m_compiler_semantic::type_resolution::definition_semantic_type(
        &db, crate_id, c_def_id,
    );
    assert!(matches!(c_ty.data(&db), TypeData::Felt));
}
