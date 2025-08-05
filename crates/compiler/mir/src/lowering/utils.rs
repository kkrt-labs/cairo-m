//! # Lowering Utilities
//!
//! This module contains shared utility functions used across the lowering
//! implementation.

use cairo_m_compiler_semantic::semantic_index::ExpressionId;
use cairo_m_compiler_semantic::type_resolution::expression_semantic_type;

use crate::{BasicBlockId, MirType};

use super::builder::MirBuilder;

impl<'a, 'db> MirBuilder<'a, 'db> {
    /// Gets the MIR type for an expression by its ID
    pub fn get_expression_type(&self, expr_id: ExpressionId) -> MirType {
        let semantic_type =
            expression_semantic_type(self.db, self.crate_id, self.file, expr_id, None);
        MirType::from_semantic_type(self.db, semantic_type)
    }

    /// Checks if we're currently in a loop context
    pub const fn in_loop(&self) -> bool {
        !self.loop_stack.is_empty()
    }

    /// Gets the current loop's continue and break targets
    pub fn current_loop_targets(&self) -> Option<(BasicBlockId, BasicBlockId)> {
        self.loop_stack.last().copied()
    }
}
