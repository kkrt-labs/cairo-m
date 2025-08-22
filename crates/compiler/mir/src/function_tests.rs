use super::*;
use crate::{Terminator, Value};

#[test]
fn test_return_value_field_with_literal() {
    let mut func = MirFunction::new("test".to_string());

    // Create a return value assignment
    let return_value_id = func.new_value_id();
    func.return_values = vec![return_value_id];

    // Set up the terminator
    func.get_basic_block_mut(func.entry_block)
        .unwrap()
        .set_terminator(Terminator::return_value(Value::integer(42)));

    // Verify the return_values field is set
    assert_eq!(func.return_values, vec![return_value_id]);

    // Verify function validation passes
    let validation_result = func.validate();
    if let Err(e) = &validation_result {
        eprintln!("Validation error: {}", e);
    }
    assert!(validation_result.is_ok());
}

#[test]
fn test_return_value_field_with_operand() {
    let mut func = MirFunction::new("test".to_string());

    // Create a value to return
    let value_id = func.new_value_id();
    func.return_values = vec![value_id];

    // Set up the terminator
    func.get_basic_block_mut(func.entry_block)
        .unwrap()
        .set_terminator(Terminator::return_value(Value::operand(value_id)));

    // Verify the return_values field is set
    assert_eq!(func.return_values, vec![value_id]);

    // Verify function validation passes
    let validation_result = func.validate();
    if let Err(e) = &validation_result {
        eprintln!("Validation error: {}", e);
    }
    assert!(validation_result.is_ok());
}
