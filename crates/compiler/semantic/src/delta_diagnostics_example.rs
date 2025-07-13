//! # Delta-based Diagnostics Example
//!
//! This example demonstrates how to use the delta-based diagnostics system
//! for efficient incremental recompilation.

#[cfg(test)]
mod example_tests {
    use salsa::Setter;

    use crate::db::tests::{crate_from_program, test_db};
    use crate::delta_diagnostics::DeltaDiagnosticsTracker;

    #[test]
    fn example_delta_diagnostics_usage() {
        let mut db = test_db();

        // Create a simple program
        let program1 = r#"
fn main() {
    let x = 42;
    let y = x + 1;
}

fn helper() {
    let z = 10;
}
"#;

        let crate_id = crate_from_program(&db, program1);
        let mut delta_tracker = DeltaDiagnosticsTracker::new();

        println!("=== First computation (all modules will be processed) ===");
        let diagnostics1 = delta_tracker.get_project_diagnostics(&db, crate_id);
        let stats1 = delta_tracker.get_cache_stats();

        println!("First run diagnostics: {} issues found", diagnostics1.len());
        println!(
            "Cache stats: {} modules tracked, {} cached",
            stats1.modules_tracked, stats1.cached_diagnostics
        );

        // Simulate no changes - should use cache
        println!("\n=== Second computation (no changes, should use cache) ===");
        let diagnostics2 = delta_tracker.get_project_diagnostics(&db, crate_id);
        let stats2 = delta_tracker.get_cache_stats();

        println!(
            "Second run diagnostics: {} issues found",
            diagnostics2.len()
        );
        println!(
            "Cache stats: {} modules tracked, {} cached",
            stats2.modules_tracked, stats2.cached_diagnostics
        );

        // Simulate a file change
        println!("\n=== Third computation (after file change) ===");
        let program2 = r#"
fn main() {
    let x = 42;
    let y = x + 1;
    let invalid_syntax // This will create a parse error
}

fn helper() {
    let z = 10;
}
"#;

        // Update the file content
        let modules = crate_id.modules(&db);
        if let Some((_, file)) = modules.iter().next() {
            file.set_text(&mut db).to(program2.to_string());
        }

        let diagnostics3 = delta_tracker.get_project_diagnostics(&db, crate_id);
        let stats3 = delta_tracker.get_cache_stats();

        println!("Third run diagnostics: {} issues found", diagnostics3.len());
        println!(
            "Cache stats: {} modules tracked, {} cached",
            stats3.modules_tracked, stats3.cached_diagnostics
        );

        // The delta system should have detected the change and recomputed diagnostics
        // Since we introduced a syntax error, there should be more diagnostics now
        assert!(!diagnostics3.is_empty(), "Should have found syntax errors");

        println!("\n=== Delta diagnostics example completed successfully! ===");
    }

    #[test]
    fn example_changed_modules_detection() {
        let mut db = test_db();
        let crate_id = crate_from_program(&db, "fn main() { let x = 42; }");
        let mut delta_tracker = DeltaDiagnosticsTracker::new();

        // Initial computation
        let _initial_diagnostics = delta_tracker.get_project_diagnostics(&db, crate_id);

        // Check which modules changed (none should have changed yet)
        let changed_before = delta_tracker.get_changed_modules(&db, crate_id);
        println!("Modules changed before update: {:?}", changed_before);

        // Update a file
        let modules = crate_id.modules(&db);
        if let Some((_, file)) = modules.iter().next() {
            file.set_text(&mut db)
                .to("fn main() { let x = 43; }".to_string());
        }

        // Check which modules changed after the update
        let changed_after = delta_tracker.get_changed_modules(&db, crate_id);
        println!("Modules changed after update: {:?}", changed_after);

        // Should detect that the main module changed
        assert!(!changed_after.is_empty(), "Should detect changed modules");
    }
}
