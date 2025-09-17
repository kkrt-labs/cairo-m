#[cfg(test)]
mod array_load_store_lowering_tests {
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
    fn test_array_index_lowers_to_load() {
        let source = r#"
        fn test() -> felt {
            let a: [felt; 3] = [1, 2, 3];
            return a[1];
        }
        "#;

        let db = TestDatabase::default();
        let crate_id = create_test_crate(&db, source);
        let module = generate_mir(&db, crate_id).expect("MIR generation failed");
        let mir_text = module.pretty_print(0);

        assert!(mir_text.contains("makefixedarray"));
        assert!(mir_text.contains("load %"), "expected load from place");
        assert!(!mir_text.to_lowercase().contains("arrayindex"));
    }

    #[test]
    fn test_array_store_lowers_to_store() {
        let source = r#"
        fn test() {
            let a: [felt; 3] = [1, 2, 3];
            a[1] = 5;
            return;
        }
        "#;

        let db = TestDatabase::default();
        let crate_id = create_test_crate(&db, source);
        let module = generate_mir(&db, crate_id).expect("MIR generation failed");
        let mir_text = module.pretty_print(0);

        assert!(mir_text.contains("store "));
        assert!(!mir_text.to_lowercase().contains("arrayinsert"));
    }
}
