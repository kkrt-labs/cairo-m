use std::collections::HashMap;
use std::path::PathBuf;

use cairo_m_compiler_parser::SourceFile;
use cairo_m_compiler_semantic::db::project_validate_semantics;
use cairo_m_ls::db::{AnalysisDatabase, ProjectCrate, ProjectCrateExt};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let mut db = AnalysisDatabase::new();

    // Create a simple test file content
    let test_content = r#"
func test_unused() {
    let x = 3;
    let x = 4;
    let x = 5;
    faoskhd();
    return ();
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

    Ok(())
}
