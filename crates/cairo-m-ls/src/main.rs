use cairo_m_ls::Backend;
use tokio::sync::mpsc;
use tower_lsp::{LspService, Server};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Note: The trace level is configurable via command-line argument, not hardcoded.
    // This allows dynamic control of logging verbosity without recompilation.
    // Usage: cairo-m-ls [debug|info|warn|error]
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let args = std::env::args().collect::<Vec<String>>();
    let default_trace_level = "info".to_string();
    let trace_level_str = args.get(1).unwrap_or(&default_trace_level);
    let trace_level = match trace_level_str.as_str() {
        "debug" => LevelFilter::DEBUG,
        "info" => LevelFilter::INFO,
        "warn" => LevelFilter::WARN,
        "error" => LevelFilter::ERROR,
        _ => LevelFilter::INFO,
    };

    let (service, socket) = LspService::build(|client| {
        // 1. Create a channel for log messages.
        // The sender is passed to the tracing layer, and the receiver is used in a dedicated task.
        let (log_sender, mut log_receiver) = mpsc::unbounded_channel();

        // 2. Clone the client for the logging task.
        let client_clone = client.clone();

        // 3. Spawn a task to listen for log messages and forward them to the LSP client.
        // This task runs on the main Tokio runtime and can safely call async functions.
        tokio::spawn(async move {
            while let Some((level, message)) = log_receiver.recv().await {
                client_clone.log_message(level, message).await;
            }
        });

        // 4. Create the LspTracingLayer with the sender part of the channel.
        let lsp_layer = cairo_m_ls::lsp_tracing::LspTracingLayer::new(log_sender);

        let directives = EnvFilter::builder()
            .with_default_directive(trace_level.into())
            .parse_lossy("");

        tracing_subscriber::registry()
            .with(lsp_layer)
            .with(directives)
            .init();

        Backend::new(client)
    })
    .finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
