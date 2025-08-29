//! Centralized instruction emission helpers for the CASM builder.
//!
//! This module provides a thin, focused API that encapsulates pushing
//! instructions, label creation, and frame write tracking. The goal is to
//! make instruction emission uniform and easy to audit.

use crate::{InstructionBuilder, Label};

/// Emission helpers wired onto the `CasmBuilder` facade.
impl super::CasmBuilder {
    /// Push an instruction into the program.
    pub(crate) fn emit_push(&mut self, instr: InstructionBuilder) {
        self.instructions.push(instr);
    }

    /// Generate a fresh label name using the builder's counter.
    pub(crate) fn emit_new_label_name(&mut self, prefix: &str) -> String {
        let label_id = self.label_counter;
        self.label_counter += 1;
        format!("{}_{}", prefix, label_id)
    }

    /// Add a label at the current instruction address.
    pub(crate) fn emit_add_label(&mut self, mut label: Label) {
        label.address = Some(self.instructions.len());
        self.labels.push(label);
    }

    /// Track that a frame region has been written.
    pub(crate) fn emit_touch(&mut self, offset: i32, size: usize) {
        if offset >= 0 {
            let end_offset = offset + size as i32 - 1;
            self.max_written_offset = self.max_written_offset.max(end_offset);
        }
    }
}
