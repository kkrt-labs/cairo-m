use lsp_types::{
    DocumentFormattingParams, FormattingOptions, TextDocumentIdentifier, TextEdit,
    WorkDoneProgressParams,
};

use super::support::{Fixture, client_capabilities, start_mock_client};

#[tokio::test]
async fn debug_formatting_output() {
    let fixture = Fixture::new();
    fixture.add_cairom_toml("test_project");
    fixture.add_file(
        "src/main.cm",
        r#"fn   main()   ->   felt{let x=5;let y=x+1;return y;}"#,
    );

    let caps = client_capabilities::base();
    let config = serde_json::json!({
        "cairo_m": {
            "debounce_ms": 0,
            "db_swap_interval_ms": 3600000
        }
    });

    let client = start_mock_client(fixture, caps, config).await.unwrap();
    client
        .open_and_wait_for_analysis("src/main.cm")
        .await
        .unwrap();

    let uri = client.file_url("src/main.cm");
    let params = DocumentFormattingParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        options: FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            ..Default::default()
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    let edits: Option<Vec<TextEdit>> = client
        .send_request::<lsp_types::request::Formatting>(params)
        .await
        .unwrap();

    if let Some(edits) = edits {
        for (i, edit) in edits.iter().enumerate() {
            println!("Edit {}: {:?}", i, edit.new_text);
            println!("Ends with newline: {}", edit.new_text.ends_with('\n'));
        }
    }

    client.shutdown().await.unwrap();
}
