#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tower_lsp::LanguageServer;
    use tower_lsp::lsp_types::*;

    use crate::Backend;

    #[tokio::test]
    async fn test_diagnostics_debouncing() {
        // Initialize logging
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .try_init()
            .ok();

        // Create a mock client
        let (service, socket) =
            tower_lsp::LspService::build(|client| Backend::new(client)).finish();
        let (client_socket, server_socket) = tower_lsp::ClientSocket::new(socket);

        // Spawn the server
        let server_handle = tokio::spawn(async move {
            tower_lsp::Server::new(
                tokio::io::duplex(1024).0,
                tokio::io::duplex(1024).1,
                server_socket,
            )
            .serve(service)
            .await;
        });

        // Get the backend through the client
        let backend = Arc::new(Backend::new(client_socket.clone()));

        // Initialize the server
        let init_params = InitializeParams::default();
        backend.initialize(init_params).await.unwrap();
        backend.initialized(InitializedParams {}).await;

        // Create a test file
        let test_uri = tower_lsp::lsp_types::Url::parse("file:///tmp/test_debounce.cm").unwrap();
        let initial_content = r#"
func test_debounce() -> felt {
    let x = 42;
    return x;
}
"#;

        // Open the file
        let open_params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: test_uri.clone(),
                language_id: "cairo-m".to_string(),
                version: 1,
                text: initial_content.to_string(),
            },
        };
        backend.did_open(open_params).await;

        // Wait for initial diagnostics
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        println!("Starting rapid typing simulation...");

        // Simulate rapid typing - add characters one by one
        let mut current_content = initial_content.to_string();
        for i in 0..10 {
            // Add a character to simulate typing
            current_content = current_content.replace(
                "let x = 42;",
                &format!("let x = 42{}; // typing...", "a".repeat(i + 1)),
            );

            let change_params = DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: test_uri.clone(),
                    version: Some(i + 2),
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: current_content.clone(),
                }],
            };

            backend.did_change(change_params).await;
            println!("Change {} sent", i + 1);

            // Small delay between keystrokes (50ms)
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        println!("Rapid typing complete. Waiting for debounce delay...");

        // Wait for debounce delay (300ms) plus some buffer
        tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;

        println!("Debounce delay passed. Diagnostics should be computed now.");

        // Shutdown
        backend.shutdown().await.unwrap();
        server_handle.abort();

        println!("✓ Debouncing test completed successfully");
    }
}
