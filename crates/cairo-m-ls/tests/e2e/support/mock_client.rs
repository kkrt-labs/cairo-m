use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use lsp_types::{
    ClientCapabilities, Diagnostic, InitializeParams, InitializeResult, InitializedParams, Url,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tower_lsp::{LspService, Server, jsonrpc};
use tracing::{debug, error, warn};

use super::Fixture;
use super::barrier::AnalysisBarrier;
use super::notification::NotificationEvent;

/// Analysis timeout duration - longer in CI environments
static ANALYSIS_TIMEOUT: OnceLock<Duration> = OnceLock::new();

/// Diagnostics timeout duration - shorter than analysis but still adjusted for CI
static DIAGNOSTICS_TIMEOUT: OnceLock<Duration> = OnceLock::new();

fn get_analysis_timeout() -> Duration {
    *ANALYSIS_TIMEOUT.get_or_init(|| {
        let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
        if is_ci {
            Duration::from_secs(60) // Even longer for CI
        } else {
            Duration::from_secs(10)
        }
    })
}

fn get_diagnostics_timeout() -> Duration {
    *DIAGNOSTICS_TIMEOUT.get_or_init(|| {
        let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
        if is_ci {
            Duration::from_secs(30) // Even longer for CI
        } else {
            Duration::from_secs(5)
        }
    })
}

/// Async-only LSP mock client for testing
pub struct MockClient {
    /// Test fixture containing files
    pub fixture: Fixture,
    /// Channel for sending messages to server
    to_server: mpsc::UnboundedSender<String>,
    /// Broadcast sender for notifications
    notification_tx: broadcast::Sender<NotificationEvent>,
    /// Persistent receiver to keep broadcast channel alive
    _persistent_rx: broadcast::Receiver<NotificationEvent>,
    /// Server task handle
    server_handle: JoinHandle<()>,
    /// Request ID counter
    next_id: Arc<AtomicU64>,
    /// Pending requests
    pending_requests: Arc<Mutex<HashMap<jsonrpc::Id, oneshot::Sender<jsonrpc::Result<Value>>>>>,
    /// Analysis synchronization barrier
    barrier: Arc<AnalysisBarrier>,
}

impl MockClient {
    /// Start a new language server instance
    pub async fn start(
        fixture: Fixture,
        client_capabilities: ClientCapabilities,
        workspace_configuration: Value,
    ) -> Result<Self> {
        // Create duplex streams for communication
        let (client_stream, server_stream) = tokio::io::duplex(1024 * 1024);
        let (server_reader, server_writer) = tokio::io::split(server_stream);
        let (client_reader, client_writer) = tokio::io::split(client_stream);

        // Create channels
        let (to_server, mut from_client) = mpsc::unbounded_channel::<String>();
        let (notification_tx, persistent_rx) = broadcast::channel::<NotificationEvent>(1000);
        let barrier = Arc::new(AnalysisBarrier::new());

        // Build language server
        let (service, socket) = LspService::build(cairo_m_ls::Backend::new).finish();

        // Spawn server task
        let server_handle = tokio::spawn(async move {
            Server::new(server_reader, server_writer, socket)
                .serve(service)
                .await;
        });

        // Spawn message writer
        let client_writer = Arc::new(Mutex::new(client_writer));
        let client_writer_clone = Arc::clone(&client_writer);
        tokio::spawn(async move {
            while let Some(msg) = from_client.recv().await {
                let mut writer = client_writer_clone.lock().await;
                if let Err(e) = writer.write_all(msg.as_bytes()).await {
                    error!("Failed to write message: {}", e);
                    break;
                }
                if let Err(e) = writer.flush().await {
                    error!("Failed to flush writer: {}", e);
                    break;
                }
            }
        });

        // Setup message reader
        let pending_requests: Arc<
            Mutex<HashMap<jsonrpc::Id, oneshot::Sender<jsonrpc::Result<Value>>>>,
        > = Arc::new(Mutex::new(HashMap::new()));
        let pending_requests_clone = Arc::clone(&pending_requests);
        let notification_tx_clone = notification_tx.clone();
        let barrier_clone = Arc::clone(&barrier);

        tokio::spawn(async move {
            let mut reader = BufReader::new(client_reader);
            let mut headers = String::new();

            loop {
                // Read headers
                headers.clear();
                loop {
                    let mut line = String::new();
                    match reader.read_line(&mut line).await {
                        Ok(0) => {
                            debug!("Server closed connection");
                            return;
                        }
                        Ok(_) => {
                            headers.push_str(&line);
                            if line == "\r\n" {
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Failed to read headers: {}", e);
                            return;
                        }
                    }
                }

                // Parse content length
                let content_length = headers
                    .lines()
                    .find(|line| line.starts_with("Content-Length:"))
                    .and_then(|line| line.split(':').nth(1))
                    .and_then(|len| len.trim().parse::<usize>().ok());

                let content_length = match content_length {
                    Some(len) => len,
                    None => {
                        warn!("Malformed Content-Length header: {}", headers);
                        continue;
                    }
                };

                if content_length == 0 {
                    continue;
                }

                // Read content
                let mut buffer = vec![0u8; content_length];
                if let Err(e) = reader.read_exact(&mut buffer).await {
                    error!("Failed to read message content: {}", e);
                    return;
                }

                // Parse JSON-RPC message
                match serde_json::from_slice::<Value>(&buffer) {
                    Ok(msg) => {
                        debug!(
                            "Received message: {}",
                            serde_json::to_string(&msg).unwrap_or_default()
                        );

                        // Handle response
                        if let Some(id) = msg.get("id") {
                            if let Some(id_num) = id.as_u64() {
                                let id = jsonrpc::Id::Number(id_num as i64);
                                let mut pending = pending_requests_clone.lock().await;
                                if let Some(tx) = pending.remove(&id) {
                                    let result = if let Some(result) = msg.get("result") {
                                        Ok(result.clone())
                                    } else if let Some(error) = msg.get("error") {
                                        match serde_json::from_value::<jsonrpc::Error>(
                                            error.clone(),
                                        ) {
                                            Ok(err) => Err(err),
                                            Err(_) => Err(jsonrpc::Error {
                                                code: jsonrpc::ErrorCode::InternalError,
                                                message: "Failed to parse error response".into(),
                                                data: None,
                                            }),
                                        }
                                    } else {
                                        Ok(Value::Null)
                                    };
                                    let _ = tx.send(result);
                                }
                            }
                        }
                        // Handle notification
                        else if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
                            let params = msg.get("params").cloned().unwrap_or(Value::Null);
                            debug!("Received notification: {} with params: {}", method, params);

                            let event = NotificationEvent::from_method_and_params(method, params);
                            debug!("Parsed notification event: {:?}", event);
                            barrier_clone.handle_notification(&event);

                            if let Err(e) = notification_tx_clone.send(event) {
                                debug!("Failed to send notification event: {:?}", e);
                            } else {
                                debug!("Successfully sent notification event");
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            "Failed to parse JSON-RPC message: {} - Content: {:?}",
                            e,
                            String::from_utf8_lossy(&buffer)
                        );
                    }
                }
            }
        });

        let client = Self {
            fixture,
            to_server,
            notification_tx,
            _persistent_rx: persistent_rx,
            server_handle,
            next_id: Arc::new(AtomicU64::new(1)),
            pending_requests,
            barrier,
        };

        // Subscribe to notifications BEFORE initialization to catch all messages
        let _initial_subscriber = client.notification_tx.subscribe();

        // Perform initialization
        client
            .initialize(client_capabilities, workspace_configuration)
            .await?;

        Ok(client)
    }

    /// Initialize the language server
    async fn initialize(
        &self,
        client_capabilities: ClientCapabilities,
        workspace_configuration: Value,
    ) -> Result<()> {
        let root_uri = self.fixture.root_url();
        let workspace_folders = vec![lsp_types::WorkspaceFolder {
            uri: root_uri.clone(),
            name: "test".to_string(),
        }];

        let mut initialization_options = HashMap::new();

        // Always set test-friendly options
        initialization_options.insert("debounce_ms".to_string(), Value::Number(0.into()));
        initialization_options.insert(
            "db_swap_interval_ms".to_string(),
            Value::Number(3600000.into()),
        ); // 1 hour

        // Override with user config
        if let Some(cairo_m_config) = workspace_configuration.get("cairo_m") {
            if let Some(debounce) = cairo_m_config.get("debounce_ms").and_then(|d| d.as_u64()) {
                initialization_options
                    .insert("debounce_ms".to_string(), Value::Number(debounce.into()));
            }
            if let Some(interval) = cairo_m_config
                .get("db_swap_interval_ms")
                .and_then(|d| d.as_u64())
            {
                initialization_options.insert(
                    "db_swap_interval_ms".to_string(),
                    Value::Number(interval.into()),
                );
            }
        }

        let params = InitializeParams {
            process_id: None,
            root_uri: Some(root_uri),
            initialization_options: Some(serde_json::to_value(initialization_options)?),
            capabilities: client_capabilities,
            trace: None,
            workspace_folders: Some(workspace_folders),
            client_info: None,
            locale: None,
            #[allow(deprecated)]
            root_path: None,
        };

        let _result: InitializeResult = self
            .send_request::<lsp_types::request::Initialize>(params)
            .await?;

        self.send_notification::<lsp_types::notification::Initialized>(InitializedParams {})
            .await?;

        // Wait for server to be ready
        let _ = timeout(Duration::from_secs(5), self.wait_for_log_message()).await;

        Ok(())
    }

    /// Send a request with retries
    pub async fn send_request<R>(&self, params: R::Params) -> Result<R::Result>
    where
        R: lsp_types::request::Request,
        R::Params: Serialize + Clone,
        R::Result: for<'de> Deserialize<'de>,
    {
        for attempt in 0..3 {
            match self.send_request_inner::<R>(params.clone()).await {
                Ok(result) => return Ok(result),
                Err(e) if attempt < 2 => {
                    warn!("Request attempt {} failed: {}, retrying...", attempt + 1, e);
                    tokio::time::sleep(Duration::from_millis(100 * (attempt + 1) as u64)).await;
                }
                Err(e) => return Err(e),
            }
        }
        unreachable!()
    }

    async fn send_request_inner<R>(&self, params: R::Params) -> Result<R::Result>
    where
        R: lsp_types::request::Request,
        R::Params: Serialize,
        R::Result: for<'de> Deserialize<'de>,
    {
        let id = jsonrpc::Id::Number(self.next_id.fetch_add(1, Ordering::SeqCst) as i64);
        let method = R::METHOD.to_string();
        let params = serde_json::to_value(params)?;

        debug!(
            "Sending request {}: method={}, params={}",
            id, method, params
        );

        let request = if params.is_null() {
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method
            })
        } else {
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params
            })
        };

        let msg = serde_json::to_string(&request)?;
        let full_msg = format!("Content-Length: {}\r\n\r\n{}", msg.len(), msg);

        // Setup response channel
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(id.clone(), tx);
        }

        // Send request
        self.to_server
            .send(full_msg)
            .with_context(|| format!("Failed to send request {}", id))?;

        // Wait for response
        match timeout(get_analysis_timeout(), rx).await {
            Ok(Ok(response)) => match response {
                Ok(value) => {
                    debug!("Request {} succeeded", id);
                    Ok(serde_json::from_value(value)?)
                }
                Err(err) => {
                    error!("Request {} failed with error: {:?}", id, err);
                    Err(anyhow::anyhow!("Request failed: {:?}", err))
                }
            },
            Ok(Err(_)) => Err(anyhow::anyhow!("Request {} cancelled", id)),
            Err(_) => {
                // Clean up pending request
                self.pending_requests.lock().await.remove(&id);
                Err(anyhow::anyhow!("Request {} timed out", id))
            }
        }
    }

    /// Send a notification
    pub async fn send_notification<N>(&self, params: N::Params) -> Result<()>
    where
        N: lsp_types::notification::Notification,
        N::Params: Serialize,
    {
        let method = N::METHOD.to_string();
        let params = serde_json::to_value(params)?;

        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let msg = serde_json::to_string(&notification)?;
        let full_msg = format!("Content-Length: {}\r\n\r\n{}", msg.len(), msg);

        self.to_server.send(full_msg)?;
        Ok(())
    }

    /// Wait for diagnostics with default timeout
    pub async fn wait_for_diagnostics_default(&self, uri: &str) -> Result<Vec<Diagnostic>> {
        self.wait_for_diagnostics(uri, get_diagnostics_timeout())
            .await
    }

    /// Wait for diagnostics for a specific URI with retries
    pub async fn wait_for_diagnostics(
        &self,
        uri: &str,
        timeout_duration: Duration,
    ) -> Result<Vec<Diagnostic>> {
        for attempt in 0..3 {
            match self.wait_for_diagnostics_inner(uri, timeout_duration).await {
                Ok(diagnostics) => return Ok(diagnostics),
                Err(e) if attempt < 2 => {
                    warn!(
                        "Diagnostics wait attempt {} failed: {}, retrying...",
                        attempt + 1,
                        e
                    );
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(e) => return Err(e),
            }
        }
        unreachable!()
    }

    async fn wait_for_diagnostics_inner(
        &self,
        uri: &str,
        timeout_duration: Duration,
    ) -> Result<Vec<Diagnostic>> {
        let start = Instant::now();

        // First check if we already have diagnostics stored in the barrier
        if let Some(diagnostics) = self.barrier.get_diagnostics(uri) {
            debug!("Found stored diagnostics for URI: {}", uri);
            return Ok(diagnostics);
        }

        // If not, wait for new notifications to arrive
        let mut rx = self.notification_tx.subscribe();

        while start.elapsed() < timeout_duration {
            match timeout(Duration::from_millis(100), rx.recv()).await {
                Ok(Ok(event)) => {
                    debug!("Received event while waiting for diagnostics: {:?}", event);
                    if let NotificationEvent::Diagnostics(params) = event {
                        if params.uri.as_str() == uri {
                            debug!("Found matching diagnostics for URI: {}", uri);
                            return Ok(params.diagnostics);
                        } else {
                            debug!(
                                "Diagnostics for different URI: {} (wanted: {})",
                                params.uri, uri
                            );
                        }
                    }
                }
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => {
                    warn!("Notification receiver lagged, resubscribing");
                    rx = self.notification_tx.subscribe();
                }
                Ok(Err(broadcast::error::RecvError::Closed)) => {
                    return Err(anyhow::anyhow!("Notification channel closed"));
                }
                Err(_) => {
                    // Timeout on recv, check stored diagnostics again in case we missed it
                    if let Some(diagnostics) = self.barrier.get_diagnostics(uri) {
                        debug!("Found stored diagnostics for URI: {} during wait", uri);
                        return Ok(diagnostics);
                    }
                    debug!("No notification received in 100ms window, continuing...");
                }
            }
        }

        Err(anyhow::anyhow!(
            "Timeout waiting for diagnostics for {}",
            uri
        ))
    }

    /// Wait for complete analysis cycle
    pub async fn wait_for_analysis_to_complete(&self) -> Result<()> {
        self.barrier
            .wait_for_complete_analysis(get_analysis_timeout())
            .await
            .map_err(|_| anyhow::anyhow!("Timeout waiting for analysis to complete"))
    }

    /// Wait for log message
    async fn wait_for_log_message(&self) -> Option<String> {
        let mut rx = self.notification_tx.subscribe();
        let start = Instant::now();

        while start.elapsed() < Duration::from_secs(2) {
            if let Ok(Ok(NotificationEvent::Log(params))) =
                timeout(Duration::from_millis(100), rx.recv()).await
            {
                return Some(params.message);
            }
        }
        None
    }

    /// Open file and wait for analysis to complete
    pub async fn open_and_wait_for_analysis(&self, path: &str) -> Result<()> {
        let uri = self.fixture.file_url(path);
        let content = self.fixture.read_file(path);

        let params = lsp_types::DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri,
                language_id: "cairo-m".to_string(),
                version: 0,
                text: content,
            },
        };

        self.send_notification::<lsp_types::notification::DidOpenTextDocument>(params)
            .await?;

        // Small delay to ensure debounced diagnostics start (even with 0ms debounce)
        // This is needed because the diagnostics are scheduled asynchronously
        tokio::time::sleep(Duration::from_millis(50)).await;

        self.wait_for_analysis_to_complete().await?;
        Ok(())
    }

    /// Get file URL
    pub fn file_url(&self, path: impl AsRef<std::path::Path>) -> Url {
        self.fixture.file_url(path)
    }

    /// Shutdown the server gracefully
    pub async fn shutdown(self) -> Result<()> {
        // Wait for any pending analysis to complete
        if self.barrier.has_completed_analysis() {
            let _ = timeout(
                Duration::from_millis(200),
                self.wait_for_analysis_to_complete(),
            )
            .await;
        }

        // Send shutdown request
        let shutdown_result = self.send_request::<lsp_types::request::Shutdown>(()).await;

        // Always abort the server handle
        self.server_handle.abort();

        shutdown_result.map(|_| ())
    }
}
