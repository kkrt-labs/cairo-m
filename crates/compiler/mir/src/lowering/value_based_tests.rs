#[cfg(test)]
mod value_based_lowering_tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use cairo_m_compiler_parser::Upcast;
    use cairo_m_compiler_semantic::db::Crate;
    use cairo_m_compiler_semantic::{File, SemanticDb};

    use crate::{MirDb, PrettyPrint, generate_mir};

    /// Test database that implements all required traits for MIR generation
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
    fn test_value_based_tuple_literal() {
        let source = r#"
        fn test() -> (felt, felt) {
            let t = (1, 2);
            return t;
        }
        "#;

        let db = TestDatabase::default();
        let crate_id = create_test_crate(&db, source);
        let module = generate_mir(&db, crate_id).expect("MIR generation failed");
        let mir_text = module.pretty_print(0);

        // After optimization, the tuple creation is eliminated and we directly return the values
        // This is correct behavior - the important thing is no memory allocation
        assert!(!mir_text.contains("framealloc"));
        // Store instructions should not be present for simple tuple creation
        assert!(!mir_text.contains("store %"));
        // No getelementptr for tuple access
        assert!(!mir_text.contains("getelementptr"));
    }

    #[test]
    fn test_value_based_struct_literal() {
        let source = r#"
        struct Point { x: felt, y: felt }

        fn test() -> Point {
            let p = Point { x: 10, y: 20 };
            return p;
        }
        "#;

        let db = TestDatabase::default();
        let crate_id = create_test_crate(&db, source);
        let module = generate_mir(&db, crate_id).expect("MIR generation failed");
        let mir_text = module.pretty_print(0);

        // After optimization, the struct creation might be optimized
        // The important thing is no memory allocation
        assert!(!mir_text.contains("framealloc"));
        // No getelementptr for struct access
        assert!(!mir_text.contains("getelementptr"));
    }

    #[test]
    fn test_value_based_tuple_index() {
        let source = r#"
        fn test() -> felt {
            let t = (42, 24);
            return t.0;
        }
        "#;

        let db = TestDatabase::default();
        let crate_id = create_test_crate(&db, source);
        let module = generate_mir(&db, crate_id).expect("MIR generation failed");
        let mir_text = module.pretty_print(0);

        // The constant folding pass will optimize extract_tuple_element(make_tuple(42, 24), 0) to just 42
        // What's important is that we don't use memory operations
        assert!(!mir_text.contains("getelementptr"));
        assert!(!mir_text.contains("framealloc"));
        // No load should be present for direct tuple element access
        assert!(!mir_text.contains("load"));
    }

    #[test]
    fn test_value_based_field_access() {
        let source = r#"
        struct Point { x: felt, y: felt }

        fn test() -> felt {
            let p = Point { x: 42, y: 24 };
            return p.x;
        }
        "#;

        let db = TestDatabase::default();
        let crate_id = create_test_crate(&db, source);
        let module = generate_mir(&db, crate_id).expect("MIR generation failed");
        let mir_text = module.pretty_print(0);

        // The constant folding pass will optimize extract_struct_field(make_struct(...), "x") to just 42
        // What's important is that we don't use memory operations
        assert!(!mir_text.contains("getelementptr"));
        assert!(!mir_text.contains("framealloc"));
        assert!(!mir_text.contains("load"));
    }

    #[test]
    fn test_empty_tuple() {
        let source = r#"
        fn test() -> () {
            let t = ();
            return t;
        }
        "#;

        let db = TestDatabase::default();
        let crate_id = create_test_crate(&db, source);
        let module = generate_mir(&db, crate_id).expect("MIR generation failed");
        let mir_text = module.pretty_print(0);

        // Empty tuple results in unreachable since there's nothing to return
        // This is expected behavior for empty tuple returns in the current implementation
        assert!(!mir_text.contains("maketuple"));
        assert!(!mir_text.contains("framealloc"));
    }

    #[test]
    fn test_assert_in_expression_position_let() {
        let source = r#"
        fn test() {
            let x = 3;
            let _ = assert(x == 3);
            return;
        }
        "#;

        let db = TestDatabase::default();
        let crate_id = create_test_crate(&db, source);
        let module = generate_mir(&db, crate_id).expect("MIR generation failed for assert in expr");
        let mir_text = module.pretty_print(0);

        // Ensure assert lowered to AssertEq rather than unresolved function call
        assert!(
            mir_text.contains("AssertEq"),
            "Expected AssertEq in MIR for assert call"
        );
        assert!(!mir_text.contains("Function 'assert' not found"));
    }

    #[test]
    fn test_assert_used_as_argument_to_unit_param() {
        let source = r#"
        fn takes_unit(x: ()) { return; }

        fn test() {
            takes_unit(assert(1 == 1));
            return;
        }
        "#;

        let db = TestDatabase::default();
        let crate_id = create_test_crate(&db, source);
        let module = generate_mir(&db, crate_id).expect("MIR generation failed for assert arg");
        let mir_text = module.pretty_print(0);

        // Assert should still be lowered when used as an argument, and no unresolved calls
        assert!(
            mir_text.contains("AssertEq"),
            "Expected AssertEq in MIR for assert call as arg"
        );
        assert!(!mir_text.contains("Function 'assert' not found"));
    }
}
