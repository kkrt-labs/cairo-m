use super::support::{Fixture, client_capabilities, start_mock_client};

#[tokio::test]
async fn test_simple_diagnostics() {
    // Create a fixture with a simple Cairo-M file
    let fixture = Fixture::new();
    fixture.add_cairom_toml("test_project");
    fixture.add_file(
        "main.cm",
        r#"
func main() {
    let _x = undefined_var; // This should produce an error
}
"#,
    );

    // Start the language server
    let caps = client_capabilities::base();
    let config = serde_json::json!({
        "cairo_m": {
            "debounce_ms": 0, // No debounce for tests
            "db_swap_interval_ms": 3600000 // 1 hour
        }
    });

    let client = start_mock_client(fixture, caps, config).await.unwrap();

    // Use the open_and_wait_for_analysis method which handles this correctly
    client.open_and_wait_for_analysis("main.cm").await.unwrap();

    // Now get the diagnostics
    let main_uri = client.file_url("main.cm").to_string();
    let diagnostics = client
        .wait_for_diagnostics(&main_uri, std::time::Duration::from_secs(5))
        .await
        .unwrap();

    // We should have at least one diagnostic for the undefined variable
    assert!(!diagnostics.is_empty(), "Expected diagnostics but got none");

    let first_diag = &diagnostics[0];
    assert!(
        first_diag.message.contains("undefined_var") || first_diag.message.contains("Undeclared")
    );

    // Graceful shutdown
    client.shutdown().await.unwrap();
}
