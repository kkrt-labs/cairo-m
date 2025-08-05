mod loader;

use loader::{load_module, print_womir_program};

fn main() {
    let program = load_module("tests/test_cases/add.wasm").unwrap();

    print_womir_program(&program);
}
