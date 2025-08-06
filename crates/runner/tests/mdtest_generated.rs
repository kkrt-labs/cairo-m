//! Individual test functions for each markdown test case
//!
//! This file includes auto-generated tests from the build script.
//! Each markdown test becomes its own test function that can be run individually.

mod common;

// Include the generated test functions
include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));
