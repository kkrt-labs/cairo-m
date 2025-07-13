#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use cairo_m_compiler_parser::SourceFile;
    use cairo_m_compiler_semantic::db::project_validate_semantics;
    use tower_lsp::lsp_types::Url;

    use crate::db::{AnalysisDatabase, ProjectCrate, ProjectCrateExt};

    #[test]
    fn test_real_math_file() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .try_init()
            .ok(); // Ignore if already initialized

        let mut db = AnalysisDatabase::new();

        // Use the real math.cm file
        let file_path = PathBuf::from("/Users/msaug/kkrt-labs/cairo-m/cairo-m-project/src/math.cm");
        let content = std::fs::read_to_string(&file_path).expect("Failed to read math.cm");

        println!("File content:\n{}", content);

        let source_file =
            SourceFile::new(&mut db, content.clone(), file_path.display().to_string());

        // Create a ProjectCrate
        let mut files = HashMap::new();
        files.insert(file_path.clone(), source_file);

        let project_crate = ProjectCrate::new(
            &mut db,
            file_path.parent().unwrap().to_path_buf(),
            "math".to_string(),
            files,
        );

        println!(
            "Created ProjectCrate with {} files",
            project_crate.files(&db).len()
        );

        // Convert to semantic crate
        let semantic_crate = project_crate.to_semantic_crate(&db);

        // Run semantic validation
        let diagnostic_collection = project_validate_semantics(&db, semantic_crate);

        println!("Found {} diagnostics:", diagnostic_collection.all().len());
        for (i, diag) in diagnostic_collection.all().iter().enumerate() {
            println!(
                "Diagnostic {}: {:?} - {} ({}:{})",
                i, diag.severity, diag.message, diag.span.start, diag.span.end
            );
        }

        // Test the URI conversion that the LSP would do
        let uri = Url::from_file_path(&file_path).expect("Valid file path");
        println!("File URI: {}", uri);

        // Test conversion to LSP diagnostic format
        use crate::diagnostics::controller::convert_cairo_diagnostic;

        if let Some(cairo_diag) = diagnostic_collection.all().first() {
            let lsp_diag = convert_cairo_diagnostic(&content, cairo_diag);
            println!("Converted LSP diagnostic: {:?}", lsp_diag);
        }
    }
}
