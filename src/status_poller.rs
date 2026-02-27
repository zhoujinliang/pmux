// status_poller.rs - Periodic status polling for tmux panes
use crate::agent_status::AgentStatus;
use crate::pane_status_tracker::PaneStatusTracker;
use crate::tmux::capture_pane;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Configuration for status polling
#[derive(Debug, Clone)]
pub struct PollerConfig {
    /// Polling interval in milliseconds
    pub interval_ms: u64,
    /// Debounce threshold for status changes
    pub debounce_threshold: u8,
    /// Minimum update interval per pane (ms)
    pub min_update_interval_ms: u64,
}

impl Default for PollerConfig {
    fn default() -> Self {
        Self {
            interval_ms: 500,
            debounce_threshold: 2,
            min_update_interval_ms: 100,
        }
    }
}

impl PollerConfig {
    /// Create default config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set polling interval
    pub fn with_interval(mut self, ms: u64) -> Self {
        self.interval_ms = ms;
        self
    }

    /// Set debounce threshold
    pub fn with_debounce(mut self, threshold: u8) -> Self {
        self.debounce_threshold = threshold;
        self
    }
}

/// Callback for status changes
pub type StatusCallback = Box<dyn Fn(&str, AgentStatus) + Send + 'static>;

/// Manages periodic status polling
pub struct StatusPoller {
    tracker: Arc<Mutex<PaneStatusTracker>>,
    config: PollerConfig,
    running: Arc<Mutex<bool>>,
    handle: Option<JoinHandle<()>>,
    callbacks: Vec<StatusCallback>,
}

impl StatusPoller {
    /// Create a new poller with default config
    pub fn new() -> Self {
        Self {
            tracker: Arc::new(Mutex::new(PaneStatusTracker::new())),
            config: PollerConfig::default(),
            running: Arc::new(Mutex::new(false)),
            handle: None,
            callbacks: Vec::new(),
        }
    }

    /// Create poller with custom config
    pub fn with_config(config: PollerConfig) -> Self {
        Self {
            tracker: Arc::new(Mutex::new(PaneStatusTracker::new())),
            config,
            running: Arc::new(Mutex::new(false)),
            handle: None,
            callbacks: Vec::new(),
        }
    }

    /// Register a callback for status changes
    pub fn on_status_change<F>(&mut self, callback: F)
    where
        F: Fn(&str, AgentStatus) + Send + 'static,
    {
        self.callbacks.push(Box::new(callback));
    }

    /// Register a pane to poll
    pub fn register_pane(&mut self, pane_id: &str) {
        if let Ok(mut tracker) = self.tracker.lock() {
            tracker.register_pane(pane_id);
        }
    }

    /// Unregister a pane
    pub fn unregister_pane(&mut self, pane_id: &str) {
        if let Ok(mut tracker) = self.tracker.lock() {
            tracker.unregister_pane(pane_id);
        }
    }

    /// Start polling in a background thread
    pub fn start(&mut self) {
        // Stop if already running
        self.stop();

        // Set running flag
        if let Ok(mut running) = self.running.lock() {
            *running = true;
        }

        let tracker = Arc::clone(&self.tracker);
        let running = Arc::clone(&self.running);
        let interval = Duration::from_millis(self.config.interval_ms);

        self.handle = Some(thread::spawn(move || {
            while let Ok(is_running) = running.lock() {
                if !*is_running {
                    break;
                }
                drop(is_running);

                // Poll all registered panes
                let pane_ids: Vec<String> = {
                    if let Ok(tracker) = tracker.lock() {
                        tracker.pane_ids()
                    } else {
                        vec![]
                    }
                };

                for pane_id in pane_ids {
                    // Capture pane content
                    if let Ok(content) = capture_pane(&pane_id) {
                        // Update status
                        if let Ok(mut tracker) = tracker.lock() {
                            tracker.update_pane(&pane_id, &content);
                        }
                    }
                }

                thread::sleep(interval);
            }
        }));
    }

    /// Stop polling
    pub fn stop(&mut self) {
        // Set running flag to false
        if let Ok(mut running) = self.running.lock() {
            *running = false;
        }

        // Wait for thread to finish
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    /// Check if poller is running
    pub fn is_running(&self) -> bool {
        self.running
            .lock()
            .map(|r| *r)
            .unwrap_or(false)
    }

    /// Get current status for a pane
    pub fn get_status(&self, pane_id: &str) -> AgentStatus {
        self.tracker
            .lock()
            .map(|t| t.get_status(pane_id))
            .unwrap_or(AgentStatus::Unknown)
    }

    /// Get all pane statuses
    pub fn get_all_statuses(&self) -> HashMap<String, AgentStatus> {
        self.tracker
            .lock()
            .map(|t| t.get_all_statuses())
            .unwrap_or_default()
    }

    /// Force update a pane's status
    pub fn force_update(&mut self, pane_id: &str, content: &str) -> bool {
        self.tracker
            .lock()
            .map(|mut t| t.update_pane(pane_id, content))
            .unwrap_or(false)
    }

    /// Get count of tracked panes
    pub fn pane_count(&self) -> usize {
        self.tracker
            .lock()
            .map(|t| t.pane_count())
            .unwrap_or(0)
    }
}

impl Default for StatusPoller {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for StatusPoller {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poller_creation() {
        let poller = StatusPoller::new();
        assert!(!poller.is_running());
        assert_eq!(poller.pane_count(), 0);
    }

    #[test]
    fn test_poller_with_config() {
        let config = PollerConfig::new()
            .with_interval(1000)
            .with_debounce(3);
        
        let poller = StatusPoller::with_config(config);
        assert!(!poller.is_running());
    }

    #[test]
    fn test_default_config() {
        let config = PollerConfig::default();
        assert_eq!(config.interval_ms, 500);
        assert_eq!(config.debounce_threshold, 2);
    }

    #[test]
    fn test_config_builder() {
        let config = PollerConfig::new()
            .with_interval(1000)
            .with_debounce(3);
        
        assert_eq!(config.interval_ms, 1000);
        assert_eq!(config.debounce_threshold, 3);
    }

    #[test]
    fn test_register_unregister_pane() {
        let mut poller = StatusPoller::new();
        
        poller.register_pane("pane-1");
        assert_eq!(poller.pane_count(), 1);
        
        poller.register_pane("pane-2");
        assert_eq!(poller.pane_count(), 2);
        
        poller.unregister_pane("pane-1");
        assert_eq!(poller.pane_count(), 1);
    }

    #[test]
    fn test_get_status_unknown() {
        let poller = StatusPoller::new();
        assert_eq!(poller.get_status("unknown-pane"), AgentStatus::Unknown);
    }

    #[test]
    fn test_callback_registration() {
        let mut poller = StatusPoller::new();
        let called = Arc::new(Mutex::new(false));
        let called_clone = Arc::clone(&called);
        
        poller.on_status_change(move |_pane_id, _status| {
            if let Ok(mut c) = called_clone.lock() {
                *c = true;
            }
        });
        
        // Just verify it compiles and runs
        assert_eq!(poller.callbacks.len(), 1);
    }

    #[test]
    fn test_start_stop() {
        let mut poller = StatusPoller::new();
        
        // Can't easily test actual threading without mocking tmux
        // Just verify the methods exist and don't panic
        poller.start();
        assert!(poller.is_running());
        
        poller.stop();
        assert!(!poller.is_running());
    }

    #[test]
    fn test_drop_stops_poller() {
        let mut poller = StatusPoller::new();
        poller.start();
        assert!(poller.is_running());
        
        // Drop should stop the poller
        drop(poller);
        // If we get here without hanging, drop worked correctly
    }
}
