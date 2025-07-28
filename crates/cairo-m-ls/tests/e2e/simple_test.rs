use super::support::{Fixture, client_capabilities, start_mock_client};

#[tokio::test]
async fn test_simple_diagnostics__() {
    // Create a fixture with a simple Cairo-M file
    let fixture = Fixture::new();
    fixture.add_cairom_toml("test_project");
    fixture.add_file(
        "src/main.cm",
        r#"
fn main() {
    let _x = undefined_var; // This should produce an error
    return;
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
    client
        .open_and_wait_for_analysis("src/main.cm")
        .await
        .unwrap();

    // Now get the diagnostics
    let main_uri = client.file_url("src/main.cm").to_string();
    let diagnostics = client
        .wait_for_diagnostics(&main_uri, std::time::Duration::from_secs(5))
        .await
        .unwrap();

    // We should have two diagnostics, one for the unused variable and one for the undeclared variable
    assert!(diagnostics[0].message.contains("Unused variable '_x"));
    assert!(diagnostics[1].message.contains("Undeclared variable"));

    // Graceful shutdown
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_simple_diagnostics_standalone() {
    // Create a fixture with a simple Cairo-M file
    let fixture = Fixture::new();
    fixture.add_file(
        "test.cm",
        r#"
fn main() {
    let _x = undefined_var; // This should produce an error
    return;
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
    client.open_and_wait_for_analysis("test.cm").await.unwrap();

    // Now get the diagnostics
    let main_uri = client.file_url("test.cm").to_string();
    let diagnostics = client
        .wait_for_diagnostics(&main_uri, std::time::Duration::from_secs(5))
        .await
        .unwrap();

    // We should have two diagnostics, one for the unused variable and one for the undeclared variable
    assert!(diagnostics[0].message.contains("Unused variable '_x"));
    assert!(diagnostics[1].message.contains("Undeclared variable"));

    // Graceful shutdown
    client.shutdown().await.unwrap();
}
