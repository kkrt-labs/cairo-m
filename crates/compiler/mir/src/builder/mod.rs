//! # MIR Builder Components
//!
//! This module contains specialized builders for different aspects of MIR construction.
//! The builders are designed to separate concerns and provide clean APIs for specific tasks.

mod cfg_builder;
mod instr_builder;

pub use cfg_builder::{CfgBuilder, CfgState};
pub use instr_builder::InstrBuilder;
