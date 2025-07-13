#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use cairo_m_compiler_parser::SourceFile;
    use cairo_m_compiler_parser::db::project_validate_parser;
    use cairo_m_compiler_semantic::db::project_validate_semantics;

    use crate::db::{AnalysisDatabase, ProjectCrate, ProjectCrateExt};

    #[test]
    fn test_diagnostics_detection() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .try_init()
            .ok(); // Ignore if already initialized

        let mut db = AnalysisDatabase::new();

        // Create a test file content with both syntax errors and semantic issues
        let test_content = r#"
func test_unused() {
    let x = 3;
    let x = 4;
    let x = 5;
    faoskhd();
    return ();
}

func invalid_syntax() {
    let ;  // Syntax error
    func    // Missing name and body
    if (true {  // Missing closing paren and body
    return 42;
}
"#;

        let file_path = PathBuf::from("/Users/msaug/kkrt-labs/cairo-m/test_diagnostics.cm");
        let source_file = SourceFile::new(
            &mut db,
            test_content.to_string(),
            file_path.display().to_string(),
        );

        // Create a ProjectCrate
        let mut files = HashMap::new();
        files.insert(file_path.clone(), source_file);

        let project_crate = ProjectCrate::new(
            &mut db,
            file_path.parent().unwrap().to_path_buf(),
            "test_diagnostics".to_string(),
            files,
        );

        println!(
            "Created ProjectCrate with {} files",
            project_crate.files(&db).len()
        );

        // First run parser validation
        let parser_crate = project_crate.to_parser_crate(&db);
        let parser_diagnostics = project_validate_parser(&db, parser_crate);

        println!(
            "Found {} parser diagnostics:",
            parser_diagnostics.all().len()
        );
        for (i, diag) in parser_diagnostics.all().iter().enumerate() {
            println!(
                "Parser Diagnostic {}: {:?} - {} ({}:{})",
                i, diag.severity, diag.message, diag.span.start, diag.span.end
            );
        }

        // Run semantic validation (should be safe now with panic handling)
        let semantic_crate = project_crate.to_semantic_crate(&db);
        let semantic_diagnostics = project_validate_semantics(&db, semantic_crate);

        println!(
            "Found {} semantic diagnostics:",
            semantic_diagnostics.all().len()
        );
        for (i, diag) in semantic_diagnostics.all().iter().enumerate() {
            println!(
                "Semantic Diagnostic {}: {:?} - {} ({}:{})",
                i, diag.severity, diag.message, diag.span.start, diag.span.end
            );
        }

        let total_diagnostics = parser_diagnostics.all().len() + semantic_diagnostics.all().len();
        println!("Total diagnostics: {}", total_diagnostics);

        // We expect at least some diagnostics (both parser and semantic)
        assert!(total_diagnostics > 0, "Expected diagnostics but found none");
        assert!(
            !parser_diagnostics.all().is_empty(),
            "Expected parser diagnostics for syntax errors"
        );

        // Test that the system doesn't panic with invalid syntax
        println!("✓ No panics occurred during validation with syntax errors");
    }
}
