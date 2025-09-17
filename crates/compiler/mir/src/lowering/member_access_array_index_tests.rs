#[cfg(test)]
mod member_access_array_index_lowering_tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use cairo_m_compiler_parser::Upcast;
    use cairo_m_compiler_semantic::db::Crate;
    use cairo_m_compiler_semantic::{File, SemanticDb};

    use crate::{generate_mir, MirDb, PrettyPrint};

    #[salsa::db]
    #[derive(Clone, Default)]
    struct TestDatabase {
        storage: salsa::Storage<Self>,
    }

    #[salsa::db]
    impl salsa::Database for TestDatabase {}

    #[salsa::db]
    impl cairo_m_compiler_parser::Db for TestDatabase {}

    #[salsa::db]
    impl SemanticDb for TestDatabase {}

    #[salsa::db]
    impl MirDb for TestDatabase {}

    impl Upcast<dyn cairo_m_compiler_parser::Db> for TestDatabase {
        fn upcast(&self) -> &(dyn cairo_m_compiler_parser::Db + 'static) {
            self
        }
        fn upcast_mut(&mut self) -> &mut (dyn cairo_m_compiler_parser::Db + 'static) {
            self
        }
    }

    impl Upcast<dyn SemanticDb> for TestDatabase {
        fn upcast(&self) -> &(dyn SemanticDb + 'static) {
            self
        }
        fn upcast_mut(&mut self) -> &mut (dyn SemanticDb + 'static) {
            self
        }
    }

    fn create_test_crate(db: &TestDatabase, source: &str) -> Crate {
        let file = File::new(db, source.to_string(), "test.cm".to_string());
        let mut modules = HashMap::new();
        modules.insert("test".to_string(), file);
        Crate::new(
            db,
            modules,
            "test".to_string(),
            PathBuf::from("."),
            "test_crate".to_string(),
        )
    }

    #[test]
    fn test_array_element_field_assignment_lowers_to_store() {
        let source = r#"
        struct Point { x: felt, y: felt }

        fn test() -> felt {
            let arr: [Point; 2] = [Point { x: 1, y: 0 }, Point { x: 2, y: 0 }];
            let i = 1;
            arr[i].x = 5;
            return arr[i].x;
        }
        "#;

        let db = TestDatabase::default();
        let crate_id = create_test_crate(&db, source);
        let module = generate_mir(&db, crate_id).expect("MIR generation failed");
        let mir_text = module.pretty_print(0);

        // Expect the field update to be performed by value (insertfield)
        assert!(
            mir_text.to_lowercase().contains("insertfield"),
            "expected insertfield for struct field update"
        );
        // And the updated struct should be stored back into the array element
        assert!(
            mir_text.contains("store "),
            "expected store back to arr[i] after field assignment"
        );
    }

    #[test]
    fn test_nested_member_field_assignment_storeback() {
        let source = r#"
        struct Inner { x: felt, y: felt }
        struct Outer { nested: Inner, t: (felt, felt) }

        fn test() -> felt {
            let arr: [Outer; 2] = [
                Outer { nested: Inner { x: 1, y: 2 }, t: (3, 4) },
                Outer { nested: Inner { x: 5, y: 6 }, t: (7, 8) }
            ];
            let i = 1;
            arr[i].nested.x = 9;
            return arr[i].nested.x;
        }
        "#;

        let db = TestDatabase::default();
        let crate_id = create_test_crate(&db, source);
        let module = generate_mir(&db, crate_id).expect("MIR generation failed");
        let mir_text = module.pretty_print(0);

        let lc = mir_text.to_lowercase();
        assert!(
            lc.contains("insertfield"),
            "expected insertfield(s) for nested field update"
        );
        assert!(
            lc.contains("\"nested\""),
            "expected wrapping insertfield on outer struct field 'nested'"
        );
        assert!(
            mir_text.contains("store "),
            "expected store back to arr[i] after nested field assignment"
        );
    }

    #[test]
    fn test_nested_tuple_index_assignment_storeback() {
        let source = r#"
        struct Outer { t: (felt, felt) }

        fn test() -> felt {
            let arr: [Outer; 2] = [
                Outer { t: (1, 2) },
                Outer { t: (3, 4) }
            ];
            let i = 1;
            arr[i].t.0 = 9;
            return arr[i].t.0;
        }
        "#;

        let db = TestDatabase::default();
        let crate_id = create_test_crate(&db, source);
        let module = generate_mir(&db, crate_id).expect("MIR generation failed");
        let mir_text = module.pretty_print(0);

        assert!(
            mir_text.to_lowercase().contains("inserttuple"),
            "expected inserttuple for tuple element update"
        );
        assert!(
            mir_text.to_lowercase().contains("insertfield"),
            "expected wrapping insertfield to update outer struct"
        );
        assert!(
            mir_text.contains("store "),
            "expected store back to arr[i] after nested tuple element assignment"
        );
    }

    #[test]
    fn test_deep_nested_member_chain_assignment_storeback() {
        let source = r#"
        struct C { c: felt }
        struct B { b: C }
        struct A { a: B }

        fn test() -> felt {
            let arr: [A; 1] = [A { a: B { b: C { c: 0 } } }];
            let i = 0;
            arr[i].a.b.c = 42;
            return arr[i].a.b.c;
        }
        "#;

        let db = TestDatabase::default();
        let crate_id = create_test_crate(&db, source);
        let module = generate_mir(&db, crate_id).expect("MIR generation failed");
        let mir_text = module.pretty_print(0).to_lowercase();

        let count_insertfield = mir_text.matches("insertfield").count();
        assert!(
            count_insertfield >= 3,
            "expected at least 3 insertfield ops (leaf, mid, outer), got {}\n{}",
            count_insertfield,
            mir_text
        );
        assert!(
            mir_text.contains("store "),
            "expected store back to arr[i] after deep nested assignment"
        );
    }

    #[test]
    fn test_mixed_nested_tuple_in_struct_assignment_storeback() {
        let source = r#"
        struct S { t: (felt, felt) }
        struct O { s: S }

        fn test() -> felt {
            let arr: [O; 1] = [O { s: S { t: (1, 2) } }];
            let i = 0;
            arr[i].s.t.1 = 13;
            return arr[i].s.t.1;
        }
        "#;

        let db = TestDatabase::default();
        let crate_id = create_test_crate(&db, source);
        let module = generate_mir(&db, crate_id).expect("MIR generation failed");
        let mir_text = module.pretty_print(0).to_lowercase();

        assert!(
            mir_text.contains("inserttuple"),
            "expected inserttuple for tuple update"
        );
        let count_insertfield = mir_text.matches("insertfield").count();
        assert!(
            count_insertfield >= 2,
            "expected at least 2 insertfield ops (wrap tuple into S, wrap S into O), got {}\n{}",
            count_insertfield,
            mir_text
        );
        assert!(mir_text.contains("store "), "expected store back to arr[i]");
    }
}
