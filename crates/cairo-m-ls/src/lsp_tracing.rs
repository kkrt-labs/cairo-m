use tokio::sync::mpsc;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

/// A tracing layer that sends log messages to the LSP client
pub struct LspTracingLayer {
    /// This sender will send log messages to a dedicated async task.
    sender: mpsc::UnboundedSender<(tower_lsp::lsp_types::MessageType, String)>,
}

impl LspTracingLayer {
    pub const fn new(
        sender: mpsc::UnboundedSender<(tower_lsp::lsp_types::MessageType, String)>,
    ) -> Self {
        Self { sender }
    }
}

impl<S> Layer<S> for LspTracingLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // Only handle our compilation-related events, skip Salsa internals
        let metadata = event.metadata();
        let target = metadata.target();

        // Skip Salsa's internal debug logs
        if target.starts_with("salsa") && !target.contains("cairo_m") {
            return;
        }

        // Only process cairo_m logs
        if !target.starts_with("cairo_m") {
            return;
        }

        // Format the message
        let mut message = String::new();
        event.record(&mut MessageVisitor(&mut message));

        // Determine level
        let level = match *metadata.level() {
            Level::ERROR => tower_lsp::lsp_types::MessageType::ERROR,
            Level::WARN => tower_lsp::lsp_types::MessageType::WARNING,
            Level::INFO => tower_lsp::lsp_types::MessageType::INFO,
            Level::DEBUG | Level::TRACE => tower_lsp::lsp_types::MessageType::LOG,
        };

        // Send the log message to the receiver task.
        // This is a non-blocking send and is safe to call from any thread.
        if let Err(e) = self.sender.send((level, message)) {
            // Can't use tracing here as it would cause recursion
            tracing::warn!("Failed to send log message to LSP client: {}", e);
        }
    }
}

struct MessageVisitor<'a>(&'a mut String);

impl<'a> tracing::field::Visit for MessageVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            *self.0 = format!("{:?}", value);
        } else {
            self.0.push_str(&format!(" {}={:?}", field.name(), value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            *self.0 = value.to_string();
        } else {
            self.0.push_str(&format!(" {}={}", field.name(), value));
        }
    }
}
