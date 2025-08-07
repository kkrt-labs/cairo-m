//! Codegen snapshot tests for mdtest cases.
//! This file automatically generates codegen snapshots for all Cairo-M code in the mdtest directory,
//! providing comprehensive coverage of real-world examples through the entire compilation pipeline.

mod common;

use cairo_m_compiler_codegen::CodeGenerator;
use cairo_m_compiler_mir::generate_mir;
use cairo_m_test_utils::{mdtest::MdTestRunner, mdtest_path};
use common::{create_test_crate, TestDatabase};
use std::path::PathBuf;

#[test]
fn test_mdtest_codegen_snapshots() {
    use insta::{assert_snapshot, glob, with_settings};

    // Use glob! to discover all markdown files in mdtest directory
    glob!(mdtest_path().to_str().unwrap(), "**/*.md", |path| {
        let db = TestDatabase::default();

        let runner = MdTestRunner::new("CASM", |source, name| {
            let crate_id = create_test_crate(&db, source, name, "mdtest");

            // First generate MIR
            let mir_module = match generate_mir(&db, crate_id) {
                Ok(module) => module,
                Err(diagnostics) => {
                    return Err(format!("MIR generation failed: {:#?}", diagnostics));
                }
            };

            // Then generate CASM code from MIR
            let mut generator = CodeGenerator::new();
            match generator.generate_module(&mir_module) {
                Ok(_) => Ok(generator.debug_instructions()),
                Err(e) => Err(format!("Code generation failed: {:#?}", e)),
            }
        })
        .with_parent_dir(true);

        let snapshots = runner.run_file(path);

        for snapshot in snapshots {
            with_settings!({
                description => format!("Codegen snapshot for mdtest: {}", snapshot.name).as_str(),
                omit_expression => true,
                snapshot_suffix => snapshot.suffix,
                prepend_module_to_snapshot => false,
            }, {
                assert_snapshot!(snapshot.content);
            });
        }
    });
}
