use anyhow::Result;
use lsp_types::ClientCapabilities;
use serde_json::Value;

use super::{Cursors, MockClient};

/// Trait for transforming LSP responses into strings for snapshot testing
#[async_trait::async_trait]
pub trait Transformer {
    /// Configure client capabilities for this transformer
    fn capabilities(base: ClientCapabilities) -> ClientCapabilities {
        base
    }

    /// Transform the language server response into a string for snapshot testing
    async fn transform(
        client: &mut MockClient,
        cursors: Cursors,
        config: Option<Value>,
    ) -> Result<String>;

    /// The default main file name for tests
    fn main_file() -> &'static str {
        "main.cm"
    }
}
