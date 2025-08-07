use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MdTestConfig {
    #[serde(default)]
    pub mdtest: MdTestSection,
    #[serde(default)]
    pub compiler: CompilerSection,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MdTestSection {
    /// Timeout in milliseconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// Maximum number of steps for execution
    #[serde(rename = "max-steps", default = "default_max_steps")]
    pub max_steps: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompilerSection {
    /// Optimization level (0-3)
    #[serde(rename = "optimization-level", default)]
    pub optimization_level: u8,
}

const fn default_timeout() -> u64 {
    5000
}

const fn default_max_steps() -> usize {
    1_000_000
}

#[derive(Debug, Clone, Default)]
pub struct TestMetadata {
    pub expected_output: Option<String>,
    pub expected_error: Option<String>,
    pub rust_equiv: Option<String>,
    pub tags: Vec<String>,
    pub ignore: Option<String>,
}

/// Represents a location in a source file
///
/// **WARNING**: The line number is approximate due to limitations in pulldown-cmark's
/// event stream. The parser does not provide exact line positions for all events,
/// so line numbers are calculated by counting newlines in text events. This means:
/// - Line numbers may be off by a few lines
/// - The exact position within a line (column) is always 0
/// - This should only be used for rough debugging/error reporting, not precise source mapping
#[derive(Debug, Clone)]
pub struct Location {
    pub file: String,
    /// Approximate line number - may be inaccurate by a few lines
    pub line: usize,
    /// Always 0 - column information is not available
    pub column: usize,
}

impl Location {
    pub const fn new(file: String, line: usize, column: usize) -> Self {
        Self { file, line, column }
    }
}
