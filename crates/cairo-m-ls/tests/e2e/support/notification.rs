use cairo_m_ls::lsp_ext::ServerStatusParams;
use lsp_types::{LogMessageParams, PublishDiagnosticsParams};
use serde_json::Value;

/// Typed notification events from the LSP server
#[derive(Debug, Clone)]
pub enum NotificationEvent {
    /// Diagnostics published for a file
    Diagnostics(PublishDiagnosticsParams),
    /// Log message from server
    Log(LogMessageParams),
    /// Server status update (analysis started/finished)
    Status(ServerStatusParams),
    /// Unknown notification (method, params)
    Unknown,
}

impl NotificationEvent {
    /// Parse a JSON-RPC notification into a typed event
    pub fn from_method_and_params(method: &str, params: Value) -> Self {
        match method {
            "textDocument/publishDiagnostics" => {
                serde_json::from_value::<PublishDiagnosticsParams>(params)
                    .map_or(Self::Unknown, Self::Diagnostics)
            }
            "window/logMessage" => {
                serde_json::from_value::<LogMessageParams>(params).map_or(Self::Unknown, Self::Log)
            }
            "cairo/serverStatus" => serde_json::from_value::<ServerStatusParams>(params)
                .map_or(Self::Unknown, Self::Status),
            _ => Self::Unknown,
        }
    }
}
