use std::collections::HashMap;
use std::time::Duration;

use cairo_m_ls::lsp_ext::ServerStatus;
use lsp_types::Diagnostic;
use tokio::sync::Notify;
use tokio::time::{Instant, timeout};

use super::notification::NotificationEvent;

/// Synchronization barrier for test reliability
pub struct AnalysisBarrier {
    /// Notifier for analysis started events
    analysis_started: Notify,
    /// Notifier for analysis finished events
    analysis_finished: Notify,
    /// Last analysis start time
    last_start: std::sync::Mutex<Option<Instant>>,
    /// Last analysis finish time
    last_finish: std::sync::Mutex<Option<Instant>>,
    /// Store diagnostics by URI
    diagnostics_store: std::sync::Mutex<HashMap<String, Vec<Diagnostic>>>,
}

impl AnalysisBarrier {
    pub fn new() -> Self {
        Self {
            analysis_started: Notify::new(),
            analysis_finished: Notify::new(),
            last_start: std::sync::Mutex::new(None),
            last_finish: std::sync::Mutex::new(None),
            diagnostics_store: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Signal that analysis has started
    pub fn signal_started(&self) {
        *self.last_start.lock().unwrap() = Some(Instant::now());
        self.analysis_started.notify_waiters();
    }

    /// Signal that analysis has finished
    pub fn signal_finished(&self) {
        *self.last_finish.lock().unwrap() = Some(Instant::now());
        self.analysis_finished.notify_waiters();
    }

    /// Wait for analysis to start
    pub async fn wait_for_start(
        &self,
        timeout_duration: Duration,
    ) -> Result<(), tokio::time::error::Elapsed> {
        timeout(timeout_duration, self.analysis_started.notified()).await
    }

    /// Wait for analysis to finish
    pub async fn wait_for_finish(
        &self,
        timeout_duration: Duration,
    ) -> Result<(), tokio::time::error::Elapsed> {
        timeout(timeout_duration, self.analysis_finished.notified()).await
    }

    /// Wait for a complete analysis cycle (start -> finish)
    pub async fn wait_for_complete_analysis(
        &self,
        timeout_duration: Duration,
    ) -> Result<(), tokio::time::error::Elapsed> {
        // First wait for start
        self.wait_for_start(timeout_duration).await?;
        // Then wait for finish
        self.wait_for_finish(timeout_duration).await?;
        Ok(())
    }

    /// Handle notification events and update barrier state
    pub fn handle_notification(&self, event: &NotificationEvent) {
        match event {
            NotificationEvent::Status(status_params) => match status_params.status {
                ServerStatus::AnalysisStarted => self.signal_started(),
                ServerStatus::AnalysisFinished => self.signal_finished(),
            },
            NotificationEvent::Diagnostics(diagnostics_params) => {
                let mut store = self.diagnostics_store.lock().unwrap();
                store.insert(
                    diagnostics_params.uri.to_string(),
                    diagnostics_params.diagnostics.clone(),
                );
            }
            NotificationEvent::Log(_) => {}
            _ => {
                tracing::error!("Received unknown notification: {:?}", event);
            }
        }
    }

    /// Check if analysis has completed at least once
    pub fn has_completed_analysis(&self) -> bool {
        let start = self.last_start.lock().unwrap();
        let finish = self.last_finish.lock().unwrap();

        match (*start, *finish) {
            (Some(start_time), Some(finish_time)) => finish_time > start_time,
            _ => false,
        }
    }

    /// Get stored diagnostics for a URI
    pub fn get_diagnostics(&self, uri: &str) -> Option<Vec<Diagnostic>> {
        let store = self.diagnostics_store.lock().unwrap();
        store.get(uri).cloned()
    }
}
