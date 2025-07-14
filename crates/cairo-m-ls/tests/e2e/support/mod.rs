#![allow(clippy::option_if_let_else)]

pub mod barrier;
pub mod cursor;
pub mod fixture;
pub mod insta;
pub mod mock_client;
pub mod notification;
pub mod transform;

pub use cursor::{Cursors, extract_cursors};
pub use fixture::Fixture;
pub use mock_client::MockClient;
pub use serde_json;
pub use transform::Transformer;

/// Base client capabilities for testing
pub mod client_capabilities {
    use lsp_types::{
        ClientCapabilities, PublishDiagnosticsClientCapabilities, TextDocumentClientCapabilities,
        WindowClientCapabilities, WorkspaceClientCapabilities,
    };

    pub fn base() -> ClientCapabilities {
        ClientCapabilities {
            workspace: Some(WorkspaceClientCapabilities::default()),
            text_document: Some(TextDocumentClientCapabilities {
                publish_diagnostics: Some(PublishDiagnosticsClientCapabilities::default()),
                ..Default::default()
            }),
            window: Some(WindowClientCapabilities::default()),
            ..Default::default()
        }
    }
}

/// Start a mock client for async tests
pub async fn start_mock_client(
    fixture: Fixture,
    client_capabilities: lsp_types::ClientCapabilities,
    workspace_configuration: serde_json::Value,
) -> Result<MockClient, anyhow::Error> {
    MockClient::start(fixture, client_capabilities, workspace_configuration).await
}

/// The sandbox macro for setting up test environments (async only)
macro_rules! sandbox {
    (
        $(files { $($file:expr => $content:expr),* $(,)? })?
        $(client_capabilities = $client_capabilities:expr;)?
        $(workspace_configuration = $overriding_workspace_configuration:expr;)?
    ) => {{
        let fixture = crate::support::Fixture::new();
        // Auto-add cairom.toml if not explicitly provided
        #[allow(clippy::useless_let_if_seq)]
        let mut has_cairom_toml = false;
        $($(
            if $file == "cairom.toml" {
                has_cairom_toml = true;
            }
            fixture.add_file($file, $content);
        )*)?

        if !has_cairom_toml {
            fixture.add_cairom_toml("test_project");
        }

        let caps = crate::support::client_capabilities::base();
        $(caps = $client_capabilities(caps);)?

        let config = serde_json::json!({
            "cairo_m": {
                "debounce_ms": 0,        // No debounce for test stability
                "db_swap_interval_ms": 3600000  // 1 hour - effectively disable during tests
            }
        });
        $(
            crate::support::merge_json(&mut config, &$overriding_workspace_configuration);
        )?

        crate::support::start_mock_client(fixture, caps, config).await.unwrap()
    }};
}

pub(crate) use sandbox;
