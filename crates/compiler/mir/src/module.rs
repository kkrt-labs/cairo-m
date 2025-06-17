//! # MIR Module
//!
//! This module defines the top-level container for MIR, representing an entire
//! compilation unit (typically a source file).

use index_vec::IndexVec;
use rustc_hash::FxHashMap;

use crate::{indent_str, FunctionId, MirFunction, PrettyPrint};

/// The MIR for an entire program module (compilation unit)
///
/// A `MirModule` contains all the functions defined in a source file,
/// along with metadata needed for linking and optimization.
///
/// # Design Notes
///
/// - Functions are stored in an `IndexVec` for efficient access by `FunctionId`
/// - Module-level constants and imports will be added in future iterations
/// - The module is designed to be easily serializable for caching
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirModule {
    /// All functions in this module, indexed by `FunctionId`
    pub functions: IndexVec<FunctionId, MirFunction>,

    /// Mapping from function names to their IDs for lookup
    /// This enables efficient name-based function resolution
    pub function_names: FxHashMap<String, FunctionId>,

    /// Optional debug information for the module
    pub debug_info: Option<ModuleDebugInfo>,
}

/// Debug information for a module
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDebugInfo {
    /// Original source file path
    pub source_path: Option<String>,

    /// Source file content hash for validation
    pub source_hash: Option<u64>,
}

impl MirModule {
    /// Creates a new empty MIR module
    pub fn new() -> Self {
        Self {
            functions: IndexVec::new(),
            function_names: FxHashMap::default(),
            debug_info: None,
        }
    }

    /// Adds a function to the module and returns its ID
    pub fn add_function(&mut self, function: MirFunction) -> FunctionId {
        let name = function.name.clone();
        let function_id = self.functions.push(function);
        self.function_names.insert(name, function_id);
        function_id
    }

    /// Gets a function by ID
    pub fn get_function(&self, id: FunctionId) -> Option<&MirFunction> {
        self.functions.get(id)
    }

    /// Gets a mutable reference to a function by ID
    pub fn get_function_mut(&mut self, id: FunctionId) -> Option<&mut MirFunction> {
        self.functions.get_mut(id)
    }

    /// Looks up a function by name
    pub fn lookup_function(&self, name: &str) -> Option<FunctionId> {
        self.function_names.get(name).copied()
    }

    /// Returns an iterator over all functions
    pub fn functions(&self) -> impl Iterator<Item = (FunctionId, &MirFunction)> {
        self.functions.iter_enumerated()
    }

    /// Returns the number of functions in this module
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Validates the module structure
    ///
    /// Checks:
    /// - All function names in the name map are valid
    /// - No duplicate function names
    /// - All functions have valid internal structure
    pub fn validate(&self) -> Result<(), String> {
        // Check function name consistency
        for (name, &func_id) in &self.function_names {
            let function = self.functions.get(func_id).ok_or_else(|| {
                format!("Function name map references invalid function ID: {func_id:?}")
            })?;

            if function.name != *name {
                return Err(format!(
                    "Function name mismatch: map has '{}', function has '{}'",
                    name, function.name
                ));
            }
        }

        // Check for duplicate names (should be caught by HashMap, but double-check)
        let mut seen_names = std::collections::HashSet::new();
        for (_, function) in self.functions() {
            if !seen_names.insert(&function.name) {
                return Err(format!("Duplicate function name: '{}'", function.name));
            }
        }

        // Validate each function's internal structure
        for (_func_id, function) in self.functions() {
            if let Err(err) = function.validate() {
                return Err(format!(
                    "Function {} validation failed: {}",
                    function.name, err
                ));
            }
        }

        Ok(())
    }
}

impl Default for MirModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PrettyPrint for MirModule {
    fn pretty_print(&self, indent: usize) -> String {
        let mut result = String::new();
        let base_indent = indent_str(indent);

        result.push_str(&format!("{base_indent}module {{\n"));

        for (func_id, function) in self.functions() {
            result.push_str(&format!("{base_indent}  // Function {func_id:?}\n"));
            result.push_str(&function.pretty_print(indent + 1));
            result.push('\n');
        }

        result.push_str(&format!("{base_indent}}}\n"));
        result
    }
}

// Arc convenience functions removed - use Arc::new(MirModule::new()) directly
