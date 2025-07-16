/// Macro for testing transformations with plain text snapshots (async only)
macro_rules! test_transform {
    ($transform_type:ty, $code:literal, @$snapshot:literal) => {{
        let (code, cursors) = crate::support::extract_cursors($code);
        let mut fixture = crate::support::Fixture::new();
        fixture.add_cairom_toml("test_project");
        fixture.add_file(<$transform_type as crate::support::Transformer>::main_file(), &code);

        let caps = <$transform_type as crate::support::Transformer>::capabilities(
            crate::support::client_capabilities::base()
        );
        let config = serde_json::json!({
            "cairo_m": { "debounce_ms": 0, "db_swap_interval_ms": 3600000 }
        });

        let mut client = crate::support::start_mock_client(fixture, caps, config).await.unwrap();
        client.open_and_wait_for_analysis(<$transform_type as crate::support::Transformer>::main_file()).await.unwrap();

        let result = <$transform_type as crate::support::Transformer>::transform(&mut client, cursors, None).await.unwrap();
        client.shutdown().await.unwrap();
        ::insta::assert_snapshot!(result, @$snapshot);
    }};

    // Version without explicit snapshot for auto-generation
    ($transform_type:ty, $code:literal) => {{
        let (code, cursors) = crate::support::extract_cursors($code);
        let fixture = crate::support::Fixture::new();
        fixture.add_cairom_toml("test_project");
        fixture.add_file(<$transform_type as crate::support::Transformer>::main_file(), &code);

        let caps = <$transform_type as crate::support::Transformer>::capabilities(
            crate::support::client_capabilities::base()
        );
        let config = serde_json::json!({
            "cairo_m": { "debounce_ms": 0, "db_swap_interval_ms": 3600000 }
        });

        let mut client = crate::support::start_mock_client(fixture, caps, config).await.unwrap();
        client.open_and_wait_for_analysis(<$transform_type as crate::support::Transformer>::main_file()).await.unwrap();

        let result = <$transform_type as crate::support::Transformer>::transform(&mut client, cursors, None).await.unwrap();
        client.shutdown().await.unwrap();
        ::insta::assert_snapshot!(result);
    }};

    ($transform_type:ty, $fixture:expr, $cursors:expr, $assert:expr) => {{
        let caps = <$transform_type as crate::support::Transformer>::capabilities(
            crate::support::client_capabilities::base(),
        );
        let config = serde_json::json!({
            "cairo_m": { "debounce_ms": 0, "db_swap_interval_ms": 3600000 }
        });

        let mut client = crate::support::start_mock_client($fixture, caps, config).await.unwrap();
        client.open_and_wait_for_analysis(<$transform_type as crate::support::Transformer>::main_file()).await.unwrap();

        let result = <$transform_type as crate::support::Transformer>::transform(&mut client, $cursors, None).await.unwrap();
        client.shutdown().await.unwrap();
        $assert(&result);
        insta::assert_snapshot!(result);
    }};
}

pub(crate) use test_transform;
