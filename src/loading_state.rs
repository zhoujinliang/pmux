// loading_state.rs - Loading and error states for pmux
use std::time::{Duration, Instant};

/// Loading spinner animation states
#[derive(Clone, Debug, PartialEq)]
pub enum SpinnerState {
    Idle,
    Spinning { start_time: Instant, frame: usize },
    Paused,
}

/// Spinner configuration
#[derive(Clone, Debug)]
pub struct SpinnerConfig {
    pub frames: Vec<&'static str>,
    pub interval_ms: u64,
}

impl Default for SpinnerConfig {
    fn default() -> Self {
        Self {
            frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            interval_ms: 80,
        }
    }
}

/// Loading spinner
#[derive(Clone, Debug)]
pub struct Spinner {
    pub state: SpinnerState,
    pub config: SpinnerConfig,
    last_update: Instant,
}

impl Default for Spinner {
    fn default() -> Self {
        Self {
            state: SpinnerState::Idle,
            config: SpinnerConfig::default(),
            last_update: Instant::now(),
        }
    }
}

impl Spinner {
    /// Create a new spinner
    pub fn new() -> Self {
        Self::default()
    }

    /// Start spinning
    pub fn start(&mut self) {
        self.state = SpinnerState::Spinning {
            start_time: Instant::now(),
            frame: 0,
        };
        self.last_update = Instant::now();
    }

    /// Stop spinning
    pub fn stop(&mut self) {
        self.state = SpinnerState::Idle;
    }

    /// Pause spinning
    pub fn pause(&mut self) {
        if matches!(self.state, SpinnerState::Spinning { .. }) {
            self.state = SpinnerState::Paused;
        }
    }

    /// Resume spinning
    pub fn resume(&mut self) {
        if matches!(self.state, SpinnerState::Paused) {
            self.state = SpinnerState::Spinning {
                start_time: Instant::now(),
                frame: 0,
            };
        }
    }

    /// Update spinner animation
    pub fn tick(&mut self) {
        if let SpinnerState::Spinning { ref mut frame, .. } = self.state {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_update);

            if elapsed >= Duration::from_millis(self.config.interval_ms) {
                *frame = (*frame + 1) % self.config.frames.len();
                self.last_update = now;
            }
        }
    }

    /// Get current frame
    pub fn current_frame(&self) -> &'static str {
        match self.state {
            SpinnerState::Spinning { frame, .. } => {
                self.config.frames.get(frame).copied().unwrap_or(" ")
            }
            _ => " ",
        }
    }

    /// Check if currently spinning
    pub fn is_spinning(&self) -> bool {
        matches!(self.state, SpinnerState::Spinning { .. })
    }

    /// Get elapsed time since start
    pub fn elapsed(&self) -> Option<Duration> {
        match self.state {
            SpinnerState::Spinning { start_time, .. } => Some(start_time.elapsed()),
            _ => None,
        }
    }
}

/// Toast notification types
#[derive(Clone, Debug, PartialEq)]
pub enum ToastType {
    Info,
    Success,
    Warning,
    Error,
}

/// Toast notification
#[derive(Clone, Debug)]
pub struct Toast {
    pub id: String,
    pub message: String,
    pub toast_type: ToastType,
    pub created_at: Instant,
    pub duration: Option<Duration>,
    pub action: Option<String>,
}

impl Toast {
    /// Create a new toast
    pub fn new(id: impl Into<String>, message: impl Into<String>, toast_type: ToastType) -> Self {
        Self {
            id: id.into(),
            message: message.into(),
            toast_type,
            created_at: Instant::now(),
            duration: Some(Duration::from_secs(5)),
            action: None,
        }
    }

    /// Create info toast
    pub fn info(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, message, ToastType::Info)
    }

    /// Create success toast
    pub fn success(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, message, ToastType::Success)
    }

    /// Create warning toast
    pub fn warning(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, message, ToastType::Warning)
    }

    /// Create error toast
    pub fn error(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(id, message, ToastType::Error)
    }

    /// Set duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Make persistent (no auto-dismiss)
    pub fn persistent(mut self) -> Self {
        self.duration = None;
        self
    }

    /// Add action button
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// Check if toast has expired
    pub fn is_expired(&self) -> bool {
        self.duration
            .map(|d| self.created_at.elapsed() > d)
            .unwrap_or(false)
    }

    /// Get remaining time
    pub fn remaining(&self) -> Option<Duration> {
        self.duration.map(|d| {
            let elapsed = self.created_at.elapsed();
            if elapsed < d {
                d - elapsed
            } else {
                Duration::ZERO
            }
        })
    }

    /// Get icon based on type
    pub fn icon(&self) -> &'static str {
        match self.toast_type {
            ToastType::Info => "ℹ️",
            ToastType::Success => "✅",
            ToastType::Warning => "⚠️",
            ToastType::Error => "❌",
        }
    }
}

/// Toast manager
#[derive(Clone, Debug, Default)]
pub struct ToastManager {
    pub toasts: Vec<Toast>,
    pub max_toasts: usize,
}

impl ToastManager {
    /// Create a new toast manager
    pub fn new() -> Self {
        Self {
            toasts: Vec::new(),
            max_toasts: 5,
        }
    }

    /// Add a toast
    pub fn add(&mut self, toast: Toast) {
        // Remove existing toast with same ID
        self.toasts.retain(|t| t.id != toast.id);

        // Add new toast at the beginning
        self.toasts.insert(0, toast);

        // Enforce max limit
        if self.toasts.len() > self.max_toasts {
            self.toasts.truncate(self.max_toasts);
        }
    }

    /// Remove a toast by ID
    pub fn remove(&mut self, id: &str) {
        self.toasts.retain(|t| t.id != id);
    }

    /// Clear all toasts
    pub fn clear(&mut self) {
        self.toasts.clear();
    }

    /// Remove expired toasts
    pub fn cleanup(&mut self) {
        self.toasts.retain(|t| !t.is_expired());
    }

    /// Get active (non-expired) toasts
    pub fn active_toasts(&self) -> Vec<&Toast> {
        self.toasts.iter().filter(|t| !t.is_expired()).collect()
    }

    /// Check if has any toasts
    pub fn has_toasts(&self) -> bool {
        !self.toasts.is_empty()
    }

    /// Get count of active toasts
    pub fn active_count(&self) -> usize {
        self.active_toasts().len()
    }
}

/// Retry configuration
#[derive(Clone, Debug)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Retry state
#[derive(Clone, Debug)]
pub struct RetryState {
    pub attempt: u32,
    pub config: RetryConfig,
    pub last_error: Option<String>,
    pub next_retry_at: Option<Instant>,
}

impl Default for RetryState {
    fn default() -> Self {
        Self {
            attempt: 0,
            config: RetryConfig::default(),
            last_error: None,
            next_retry_at: None,
        }
    }
}

impl RetryState {
    /// Create new retry state
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom config
    pub fn with_config(config: RetryConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// Record an attempt
    pub fn record_attempt(&mut self, error: impl Into<String>) {
        self.attempt += 1;
        self.last_error = Some(error.into());

        if self.can_retry() {
            let delay_ms = self.calculate_delay();
            self.next_retry_at = Some(Instant::now() + Duration::from_millis(delay_ms));
        }
    }

    /// Calculate delay for next retry using exponential backoff
    fn calculate_delay(&self) -> u64 {
        let delay = self.config.base_delay_ms as f64
            * self.config.backoff_multiplier.powi(self.attempt as i32 - 1);
        let delay = delay.min(self.config.max_delay_ms as f64) as u64;
        delay
    }

    /// Check if can retry
    pub fn can_retry(&self) -> bool {
        self.attempt < self.config.max_attempts
    }

    /// Check if it's time to retry
    pub fn should_retry_now(&self) -> bool {
        self.next_retry_at
            .map(|t| Instant::now() >= t)
            .unwrap_or(true)
    }

    /// Reset retry state
    pub fn reset(&mut self) {
        self.attempt = 0;
        self.last_error = None;
        self.next_retry_at = None;
    }

    /// Get progress (0.0 to 1.0)
    pub fn progress(&self) -> f64 {
        self.attempt as f64 / self.config.max_attempts as f64
    }

    /// Get formatted status message
    pub fn status_message(&self) -> String {
        if self.can_retry() {
            format!(
                "Attempt {}/{} failed. Retrying...",
                self.attempt, self.config.max_attempts
            )
        } else {
            format!(
                "Failed after {} attempts: {}",
                self.attempt,
                self.last_error.as_deref().unwrap_or("Unknown error")
            )
        }
    }
}

/// Network connection state
#[derive(Clone, Debug, PartialEq)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Reconnecting { attempt: u32 },
    Unknown,
}

/// Network status monitor
#[derive(Clone, Debug)]
pub struct NetworkStatus {
    pub state: ConnectionState,
    pub last_connected: Option<Instant>,
    pub last_disconnected: Option<Instant>,
}

impl Default for NetworkStatus {
    fn default() -> Self {
        Self {
            state: ConnectionState::Unknown,
            last_connected: None,
            last_disconnected: None,
        }
    }
}

impl NetworkStatus {
    /// Create new network status
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark as connected
    pub fn mark_connected(&mut self) {
        self.state = ConnectionState::Connected;
        self.last_connected = Some(Instant::now());
    }

    /// Mark as disconnected
    pub fn mark_disconnected(&mut self) {
        self.state = ConnectionState::Disconnected;
        self.last_disconnected = Some(Instant::now());
    }

    /// Mark as reconnecting
    pub fn mark_reconnecting(&mut self, attempt: u32) {
        self.state = ConnectionState::Reconnecting { attempt };
    }

    /// Check if currently connected
    pub fn is_connected(&self) -> bool {
        matches!(self.state, ConnectionState::Connected)
    }

    /// Get disconnection duration
    pub fn disconnection_duration(&self) -> Option<Duration> {
        if !self.is_connected() {
            self.last_disconnected.map(|t| t.elapsed())
        } else {
            None
        }
    }

    /// Get status message
    pub fn status_message(&self) -> &'static str {
        match self.state {
            ConnectionState::Connected => "Connected",
            ConnectionState::Disconnected => "Disconnected",
            ConnectionState::Reconnecting { .. } => "Reconnecting...",
            ConnectionState::Unknown => "Checking connection...",
        }
    }

    /// Get status icon
    pub fn status_icon(&self) -> &'static str {
        match self.state {
            ConnectionState::Connected => "🟢",
            ConnectionState::Disconnected => "🔴",
            ConnectionState::Reconnecting { .. } => "🟡",
            ConnectionState::Unknown => "⚪",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_lifecycle() {
        let mut spinner = Spinner::new();
        assert!(!spinner.is_spinning());

        spinner.start();
        assert!(spinner.is_spinning());
        assert!(spinner.elapsed().is_some());

        spinner.pause();
        assert!(!spinner.is_spinning());

        spinner.resume();
        assert!(spinner.is_spinning());

        spinner.stop();
        assert!(!spinner.is_spinning());
    }

    #[test]
    fn test_spinner_frames() {
        let spinner = Spinner::new();
        assert_eq!(spinner.current_frame(), " ");
    }

    #[test]
    fn test_toast_creation() {
        let toast = Toast::info("test-1", "Test message");
        assert_eq!(toast.toast_type, ToastType::Info);
        assert_eq!(toast.message, "Test message");
        assert_eq!(toast.icon(), "ℹ️");
        assert!(!toast.is_expired());
    }

    #[test]
    fn test_toast_types() {
        let info = Toast::info("i", "info");
        let success = Toast::success("s", "success");
        let warning = Toast::warning("w", "warning");
        let error = Toast::error("e", "error");

        assert_eq!(info.icon(), "ℹ️");
        assert_eq!(success.icon(), "✅");
        assert_eq!(warning.icon(), "⚠️");
        assert_eq!(error.icon(), "❌");
    }

    #[test]
    fn test_toast_manager() {
        let mut manager = ToastManager::new();
        assert!(!manager.has_toasts());

        manager.add(Toast::info("1", "First"));
        assert!(manager.has_toasts());
        assert_eq!(manager.active_count(), 1);

        manager.add(Toast::info("2", "Second"));
        assert_eq!(manager.active_count(), 2);

        // Adding with same ID replaces
        manager.add(Toast::info("1", "Updated"));
        assert_eq!(manager.active_count(), 2);

        manager.remove("1");
        assert_eq!(manager.active_count(), 1);

        manager.clear();
        assert!(!manager.has_toasts());
    }

    #[test]
    fn test_retry_state() {
        let mut retry = RetryState::new();
        assert!(retry.can_retry());
        assert_eq!(retry.attempt, 0);

        retry.record_attempt("Error 1");
        assert_eq!(retry.attempt, 1);
        assert!(retry.can_retry());

        retry.record_attempt("Error 2");
        assert_eq!(retry.attempt, 2);
        assert!(retry.can_retry());

        retry.record_attempt("Error 3");
        assert_eq!(retry.attempt, 3);
        assert!(!retry.can_retry());

        retry.reset();
        assert_eq!(retry.attempt, 0);
        assert!(retry.can_retry());
    }

    #[test]
    fn test_network_status() {
        let mut status = NetworkStatus::new();
        assert_eq!(status.state, ConnectionState::Unknown);

        status.mark_connected();
        assert!(status.is_connected());
        assert_eq!(status.status_message(), "Connected");

        status.mark_disconnected();
        assert!(!status.is_connected());
        assert_eq!(status.status_message(), "Disconnected");

        status.mark_reconnecting(1);
        assert_eq!(status.status_message(), "Reconnecting...");
    }

    #[test]
    fn test_persistent_toast() {
        let toast = Toast::error("e", "Error").persistent();
        assert!(toast.duration.is_none());
        assert!(!toast.is_expired());
    }

    #[test]
    fn test_toast_with_action() {
        let toast = Toast::error("e", "Error").with_action("Retry");
        assert_eq!(toast.action, Some("Retry".to_string()));
    }
}
