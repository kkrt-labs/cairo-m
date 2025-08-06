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

#[derive(Debug, Clone)]
pub struct Location {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl Location {
    pub const fn new(file: String, line: usize, column: usize) -> Self {
        Self { file, line, column }
    }
}
