use crate::assert_parses_files;

#[test]
fn test_cases_files() {
    // Parse all .cm files in the test_cases directory
    assert_parses_files!("tests/test_cases");
}

#[test]
fn test_cases_control_flow() {
    // Parse only control flow test files
    assert_parses_files!("tests/test_cases/control_flow");
}
