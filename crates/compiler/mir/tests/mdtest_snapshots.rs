//! MIR snapshot tests for mdtest cases.
//! This file automatically generates MIR snapshots for all Cairo-M code in the mdtest directory,
//! providing comprehensive coverage of real-world examples through the MIR generation pipeline.

mod common;

use cairo_m_compiler_mir::{generate_mir, PrettyPrint};
use cairo_m_test_utils::{mdtest::MdTestRunner, mdtest_path};
use common::{create_test_crate, TestDatabase};

#[test]
fn test_mdtest_mir_snapshots() {
    use insta::{assert_snapshot, glob, with_settings};

    // Use glob! to discover all markdown files in mdtest directory
    glob!(mdtest_path().to_str().unwrap(), "**/*.md", |path| {
        let db = TestDatabase::default();

        let runner = MdTestRunner::new("MIR", |source, name| {
            let crate_id = create_test_crate(&db, source, name, "mdtest");

            match generate_mir(&db, crate_id) {
                Ok(module) => Ok(module.pretty_print(0)),
                Err(diagnostics) => Err(format!(
                    "MIR generation failed with diagnostics:\n{:#?}",
                    diagnostics
                )),
            }
        });

        let snapshots = runner.run_file(path);

        for snapshot in snapshots {
            with_settings!({
                description => format!("MIR snapshot for mdtest: {}", snapshot.name).as_str(),
                omit_expression => true,
                snapshot_suffix => snapshot.suffix,
                prepend_module_to_snapshot => false,
            }, {
                assert_snapshot!(snapshot.content);
            });
        }
    });
}
