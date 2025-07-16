use anyhow::Result;
use lsp_types::{
    ClientCapabilities, GotoDefinitionParams, GotoDefinitionResponse, TextDocumentIdentifier,
    TextDocumentPositionParams,
};
use serde_json::Value;

use super::support::{Cursors, MockClient, Transformer};

/// Transformer for testing goto definition
pub struct GotoDefinition;

const NO_DEFINITION_FOUND: &str = "No definition found";

#[async_trait::async_trait]
impl Transformer for GotoDefinition {
    fn capabilities(mut base: ClientCapabilities) -> ClientCapabilities {
        // Enable goto definition capability
        if let Some(ref mut text_document) = base.text_document {
            text_document.definition = Some(Default::default());
        }
        base
    }

    async fn transform(
        client: &mut MockClient,
        cursors: Cursors,
        _config: Option<Value>,
    ) -> Result<String> {
        let position = cursors.assert_single_caret();

        // Helper function to sanitize paths for stable snapshots
        let sanitize_path = |path: &str| -> String {
            path.rfind('/').map_or_else(
                || path.to_string(),
                |pos| format!("<TEMP_DIR>/{}", &path[pos + 1..]),
            )
        };

        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: client.file_url(Self::main_file()),
                },
                position,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        // Open the file and wait for analysis first
        client.open_and_wait_for_analysis(Self::main_file()).await?;

        let response: Option<GotoDefinitionResponse> = client
            .send_request::<lsp_types::request::GotoDefinition>(params)
            .await?;

        // Format response for snapshot testing
        Ok(match response {
            Some(GotoDefinitionResponse::Scalar(location)) => {
                format!(
                    "Definition at {}:{}:{}",
                    sanitize_path(location.uri.path()),
                    location.range.start.line + 1, // Convert to 1-based for user display
                    location.range.start.character + 1
                )
            }
            Some(GotoDefinitionResponse::Array(locations)) => {
                if locations.is_empty() {
                    NO_DEFINITION_FOUND.to_string()
                } else {
                    let mut output = String::new();
                    for loc in locations {
                        output.push_str(&format!(
                            "Definition at {}:{}:{}\n",
                            sanitize_path(loc.uri.path()),
                            loc.range.start.line + 1, // Convert to 1-based
                            loc.range.start.character + 1
                        ));
                    }
                    output.trim_end().to_string() // Remove trailing newline
                }
            }
            Some(GotoDefinitionResponse::Link(links)) => {
                if links.is_empty() {
                    NO_DEFINITION_FOUND.to_string()
                } else {
                    let mut output = String::new();
                    for link in links {
                        output.push_str(&format!(
                            "Definition at {}:{}:{}\n",
                            sanitize_path(link.target_uri.path()),
                            link.target_range.start.line + 1, // Convert to 1-based
                            link.target_range.start.character + 1
                        ));
                    }
                    output.trim_end().to_string() // Remove trailing newline
                }
            }
            None => NO_DEFINITION_FOUND.to_string(),
        })
    }
}

#[cfg(test)]
mod local_definitions;

#[cfg(test)]
mod cross_file_definitions;
