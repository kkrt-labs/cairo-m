//! Main test runner for MIR generation.

use insta::assert_snapshot;

use crate::ir_generation::tests::test_harness::{MirTest, check_mir};

/// A macro to define a MIR test case. It loads a source file,
/// runs MIR generation, checks embedded assertions, and snapshots the output.
macro_rules! mir_test {
    ($test_name:ident, $subdir:expr) => {
        #[test]
        fn $test_name() {
            // Construct the path to the test source file.
            let path = concat!("test_cases/", $subdir, "/", stringify!($test_name), ".cm");

            // Load the test case, which also parses assertions.
            let test = MirTest::load(path);

            // Generate MIR from the source code.
            let mir_output = check_mir(&test.source);


            // Use insta to snapshot the entire MIR test output.
            // The snapshot name is derived from the subdirectory and test name for clarity.
            let snapshot_content = format!(
                "---\nsource: {}\nexpression: mir_output\n---\nFixture: {}.cm\n============================================================\nSource code:\n{}\n============================================================\nGenerated MIR:\n{}",
                file!(),
                stringify!($test_name),
                test.source,
                mir_output.mir_string
            );
            assert_snapshot!(concat!($subdir, "_", stringify!($test_name)), snapshot_content);

            // Validate assertions after snapshotting (to get visual output).
            test.check_assertions(&mir_output);
        }
    };
}

// ====== Test Groups ======

// --- Simple Functions ---
mir_test!(function_simple, "simple");
mir_test!(function_with_let, "simple");
mir_test!(function_with_params, "simple");

// --- Expressions ---
mir_test!(unary_ops, "expressions");
mir_test!(binary_ops, "expressions");
mir_test!(comparison_ops, "expressions");
mir_test!(compound_expr, "expressions");
mir_test!(operator_precedence, "expressions");

// --- Control Flow ---
mir_test!(if_else, "control_flow");
mir_test!(if_partial_return, "control_flow");
mir_test!(if_both_return, "control_flow");
mir_test!(if_no_else, "control_flow");
mir_test!(nested_if, "control_flow");
mir_test!(unreachable_after_return, "control_flow");
mir_test!(simple_while, "control_flow");
mir_test!(infinite_loop, "control_flow");
mir_test!(nested_loops, "control_flow");
mir_test!(loop_with_breaks, "control_flow");

// --- Functions ---
mir_test!(call_as_statement, "functions");
mir_test!(call_with_return, "functions");
mir_test!(multiple_functions, "functions");
mir_test!(fib, "functions");
mir_test!(fib_loop, "functions");
mir_test!(return_values, "functions");

// --- Variables ---
mir_test!(assignment, "variables");
mir_test!(reassignment_from_var, "variables");

// --- Optimizations ---
mir_test!(unused_variable_elimination, "optimizations");

// --- Aggregates (Structs/Tuples) ---
mir_test!(struct_literal, "aggregates");
mir_test!(struct_access_mut, "aggregates");
mir_test!(struct_access_read, "aggregates");
mir_test!(tuple_literal_and_access, "aggregates");
// TODO this theoritically works but we dont have proper memory allocation mechanisms.
mir_test!(array_access, "aggregates");

// --- Tuple Destructuring ---
mir_test!(tuple_destructuring, "expressions");
