use anyhow::Result;
use lsp_types::Diagnostic;
use serde_json::Value;

use super::support::{Cursors, MockClient, Transformer};
use crate::support::insta::test_transform;
/// Simple diagnostics transformer for testing
pub struct DiagnosticsTransformer;

#[async_trait::async_trait]
impl Transformer for DiagnosticsTransformer {
    async fn transform(
        client: &mut MockClient,
        _cursors: Cursors,
        _config: Option<Value>,
    ) -> Result<String> {
        // Open the main file and wait for analysis
        client.open_and_wait_for_analysis(Self::main_file()).await?;

        // Get diagnostics
        let main_uri = client.file_url(Self::main_file()).to_string();

        let diagnostics = client
            .wait_for_diagnostics(&main_uri, std::time::Duration::from_secs(5))
            .await?;

        // Format diagnostics for snapshot testing
        Ok(format_diagnostics(&diagnostics))
    }
}

fn format_diagnostics(diagnostics: &[Diagnostic]) -> String {
    if diagnostics.is_empty() {
        return "No diagnostics".to_string();
    }

    let mut result = String::new();
    for diag in diagnostics {
        result.push_str(&format!(
            "{}:{}-{}: {}: {}\n",
            diag.range.start.line,
            diag.range.start.character,
            diag.range.end.character,
            diag.severity
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "Unknown".to_string()),
            diag.message
        ));
    }
    result.trim_end().to_string()
}

#[tokio::test]
async fn test_basic_diagnostics() {
    test_transform!(
        DiagnosticsTransformer,
        r#"
fn main() {
    let _x = undefined_var; // This should produce an error
}
"#
    );
}

#[tokio::test]
async fn test_no_errors() {
    test_transform!(
        DiagnosticsTransformer,
        r#"
fn main() {
    let x = 42;
    let _y = x + 1;
}
"#
    );
}
