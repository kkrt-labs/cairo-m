use std::sync::Arc;

use tower_lsp::Client;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

/// A tracing layer that sends log messages to the LSP client
pub struct LspTracingLayer {
    client: Arc<Client>,
}

impl LspTracingLayer {
    pub const fn new(client: Arc<Client>) -> Self {
        Self { client }
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

        // Send to LSP client based on level
        let client = self.client.clone();
        let level = match *metadata.level() {
            Level::ERROR => tower_lsp::lsp_types::MessageType::ERROR,
            Level::WARN => tower_lsp::lsp_types::MessageType::WARNING,
            Level::INFO => tower_lsp::lsp_types::MessageType::INFO,
            Level::DEBUG | Level::TRACE => tower_lsp::lsp_types::MessageType::LOG,
        };

        // Send asynchronously, but only if we're in a Tokio context
        // Background threads without Tokio runtime will skip logging to LSP
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                client.log_message(level, message).await;
            });
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
