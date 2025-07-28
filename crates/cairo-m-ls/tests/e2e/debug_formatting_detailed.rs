use lsp_types::{
    DocumentFormattingParams, FormattingOptions, TextDocumentIdentifier, TextEdit,
    WorkDoneProgressParams,
};

use super::support::{Fixture, client_capabilities, start_mock_client};

#[tokio::test]
async fn debug_formatting_detailed() {
    let fixture = Fixture::new();
    fixture.add_cairom_toml("test_project");

    let code = r#"fn test(x:felt,y:felt)->felt{if x==0{return y;}else{return x+y;}}"#;
    fixture.add_file("src/test.cm", code);

    let caps = client_capabilities::base();
    let config = serde_json::json!({
        "cairo_m": {
            "debounce_ms": 0,
            "db_swap_interval_ms": 3600000
        }
    });

    let client = start_mock_client(fixture, caps, config).await.unwrap();

    // Open file and wait for it to be analyzed
    client
        .open_and_wait_for_analysis("src/test.cm")
        .await
        .unwrap();

    // Give it a moment to fully process
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let uri = client.file_url("src/test.cm");
    let params = DocumentFormattingParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        options: FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            ..Default::default()
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    println!("Sending formatting request...");
    let edits: Option<Vec<TextEdit>> = client
        .send_request::<lsp_types::request::Formatting>(params)
        .await
        .unwrap();

    println!("Response: {:?}", edits);

    if let Some(edits) = edits {
        println!("Number of edits: {}", edits.len());
        for (i, edit) in edits.iter().enumerate() {
            println!("Edit {}: Range = {:?}", i, edit.range);
            println!("Edit {}: Text = '{}'", i, edit.new_text);
            println!("Edit {}: Text length = {}", i, edit.new_text.len());
        }
    } else {
        println!("No edits returned!");
    }

    client.shutdown().await.unwrap();
}
