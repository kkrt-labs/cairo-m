use anyhow::Result;
use lsp_types::{
    ClientCapabilities, Hover, HoverClientCapabilities, HoverParams, MarkupKind,
    TextDocumentIdentifier, TextDocumentPositionParams,
};
use serde_json::Value;

use super::support::{Cursors, MockClient, Transformer};

/// Transformer for testing hover functionality
#[derive(Debug, Clone, Copy)]
pub struct HoverTransformer;

#[async_trait::async_trait]
impl Transformer for HoverTransformer {
    fn capabilities(mut base: ClientCapabilities) -> ClientCapabilities {
        // Enable hover capability with markdown support
        if let Some(ref mut text_document) = base.text_document {
            text_document.hover = Some(HoverClientCapabilities {
                dynamic_registration: Some(false),
                content_format: Some(vec![MarkupKind::Markdown, MarkupKind::PlainText]),
            });
        }
        base
    }

    async fn transform(
        client: &mut MockClient,
        cursors: Cursors,
        _config: Option<Value>,
    ) -> Result<String> {
        let position = cursors.assert_single_caret();

        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: client.file_url(Self::main_file()),
                },
                position,
            },
            work_done_progress_params: Default::default(),
        };

        // Open the file and wait for analysis first
        client.open_and_wait_for_analysis(Self::main_file()).await?;

        let response: Option<Hover> = client
            .send_request::<lsp_types::request::HoverRequest>(params)
            .await?;

        // Format hover response for snapshot testing
        Ok(match response {
            Some(hover) => {
                let content = match hover.contents {
                    lsp_types::HoverContents::Scalar(markup) => match markup {
                        lsp_types::MarkedString::String(s) => s,
                        lsp_types::MarkedString::LanguageString(ls) => {
                            format!("```{}\n{}\n```", ls.language, ls.value)
                        }
                    },
                    lsp_types::HoverContents::Array(markups) => markups
                        .into_iter()
                        .map(|m| match m {
                            lsp_types::MarkedString::String(s) => s,
                            lsp_types::MarkedString::LanguageString(ls) => {
                                format!("```{}\n{}\n```", ls.language, ls.value)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                    lsp_types::HoverContents::Markup(markup_content) => match markup_content.kind {
                        MarkupKind::PlainText => markup_content.value,
                        MarkupKind::Markdown => markup_content.value,
                    },
                };

                if let Some(range) = hover.range {
                    format!(
                        "Hover at {}:{}-{}:{}\n{}",
                        range.start.line,
                        range.start.character,
                        range.end.line,
                        range.end.character,
                        content
                    )
                } else {
                    content
                }
            }
            None => "No hover info".to_string(),
        })
    }
}

#[cfg(test)]
mod type_hover;

#[cfg(test)]
mod cross_file_hover;
