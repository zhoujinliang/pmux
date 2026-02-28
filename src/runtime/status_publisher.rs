//! status_publisher.rs - Publishes AgentStateChange to EventBus (event-driven, no polling)
//!
//! Replaces the polling-based StatusPoller with an event-driven architecture.
//! Status detection is triggered when terminal content changes, not on a timer.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::agent_status::AgentStatus;
use crate::runtime::event_bus::{
    AgentStateChange, EventBus, Notification, NotificationType, RuntimeEvent,
};
use crate::shell_integration::ShellPhaseInfo;
use crate::status_detector::{DebouncedStatusTracker, ProcessStatus, StatusDetector};

/// Publishes agent status changes to EventBus (event-driven, no polling loop).
///
/// Unlike the old StatusPoller which polled every 500ms, this version is triggered
/// by terminal content changes. Call `check_status()` whenever new terminal output
/// is processed.
#[derive(Clone)]
pub struct StatusPublisher {
    event_bus: Arc<EventBus>,
    tracker: Arc<Mutex<HashMap<String, DebouncedStatusTracker>>>,
    detector: StatusDetector,
}

impl StatusPublisher {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            event_bus,
            tracker: Arc::new(Mutex::new(HashMap::new())),
            detector: StatusDetector::new(),
        }
    }

    /// Register a pane for status tracking.
    pub fn register_pane(&self, pane_id: &str) {
        if let Ok(mut t) = self.tracker.lock() {
            t.insert(pane_id.to_string(), DebouncedStatusTracker::new());
        }
    }

    /// Unregister a pane from status tracking.
    pub fn unregister_pane(&self, pane_id: &str) {
        if let Ok(mut t) = self.tracker.lock() {
            t.remove(pane_id);
        }
    }

    /// Check status for a specific pane and publish if changed.
    ///
    /// Call this when terminal content changes (e.g., after processing new PTY output).
    ///
    /// # Arguments
    /// * `pane_id` - The pane to check
    /// * `process_status` - Process lifecycle status from runtime
    /// * `shell_info` - Optional shell phase info from OSC 133 markers
    /// * `content` - Current terminal content for text-based fallback detection
    ///
    /// # Returns
    /// `true` if status changed and was published
    pub fn check_status(
        &self,
        pane_id: &str,
        process_status: ProcessStatus,
        shell_info: Option<ShellPhaseInfo>,
        content: &str,
    ) -> bool {
        let mut tracker_guard = match self.tracker.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };

        let tracker = match tracker_guard.get_mut(pane_id) {
            Some(t) => t,
            None => return false,
        };

        // Detect status with full context (process > OSC 133 > text fallback)
        let new_status = self.detector.detect(process_status, shell_info, content);

        // Check if status changed (with debouncing)
        let changed = tracker.update_with_status(new_status);

        if changed {
            let current_status = tracker.current_status();
            let agent_id = pane_id.split(':').next().unwrap_or(pane_id).to_string();

            // Publish state change event
            self.event_bus
                .publish(RuntimeEvent::AgentStateChange(AgentStateChange {
                    agent_id: agent_id.clone(),
                    pane_id: Some(pane_id.to_string()),
                    state: current_status,
                }));

            // Publish notification for urgent states
            if current_status.is_urgent() {
                let notif_type = match current_status {
                    AgentStatus::Error => NotificationType::Error,
                    AgentStatus::Waiting => NotificationType::WaitingInput,
                    AgentStatus::WaitingConfirm => NotificationType::WaitingConfirm,
                    _ => return true,
                };
                self.event_bus
                    .publish(RuntimeEvent::Notification(Notification {
                        agent_id,
                        pane_id: Some(pane_id.to_string()),
                        message: current_status.display_text().to_string(),
                        notif_type,
                    }));
            }
        }

        changed
    }

    /// Get current status for a pane.
    pub fn current_status(&self, pane_id: &str) -> AgentStatus {
        if let Ok(t) = self.tracker.lock() {
            if let Some(tracker) = t.get(pane_id) {
                return tracker.current_status();
            }
        }
        AgentStatus::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell_integration::{ShellPhase, ShellPhaseInfo};
    use crate::status_detector::ProcessStatus;

    #[test]
    fn test_status_publisher_new() {
        let bus = Arc::new(EventBus::new(8));
        let pub_ = StatusPublisher::new(bus);
        pub_.register_pane("%0");
        pub_.unregister_pane("%0");
    }

    #[test]
    fn test_check_status_detects_running() {
        let bus = Arc::new(EventBus::new(8));
        let pub_ = StatusPublisher::new(Arc::clone(&bus));
        pub_.register_pane("pane-1");

        // First call sets pending (debounce)
        let changed1 = pub_.check_status("pane-1", ProcessStatus::Running, None, "AI is thinking");
        assert!(!changed1);

        // Second call with same status commits
        let changed2 = pub_.check_status(
            "pane-1",
            ProcessStatus::Running,
            None,
            "AI is still thinking",
        );
        assert!(changed2);
        assert_eq!(pub_.current_status("pane-1"), AgentStatus::Running);
    }

    #[test]
    fn test_check_status_with_shell_phase() {
        let bus = Arc::new(EventBus::new(8));
        let pub_ = StatusPublisher::new(Arc::clone(&bus));
        pub_.register_pane("pane-1");

        let info = ShellPhaseInfo {
            phase: ShellPhase::Running,
            last_post_exec_exit_code: None,
        };

        // Running phase should return Running
        let changed1 =
            pub_.check_status("pane-1", ProcessStatus::Running, Some(info), "any content");
        assert!(!changed1);

        let changed2 = pub_.check_status(
            "pane-1",
            ProcessStatus::Running,
            Some(info),
            "still running",
        );
        assert!(changed2);
        assert_eq!(pub_.current_status("pane-1"), AgentStatus::Running);
    }

    #[test]
    fn test_check_status_error_bypasses_debounce() {
        let bus = Arc::new(EventBus::new(8));
        let pub_ = StatusPublisher::new(Arc::clone(&bus));
        pub_.register_pane("pane-1");

        // Set to Running first
        pub_.check_status("pane-1", ProcessStatus::Running, None, "AI is thinking");
        pub_.check_status("pane-1", ProcessStatus::Running, None, "AI is thinking");
        assert_eq!(pub_.current_status("pane-1"), AgentStatus::Running);

        // Error should bypass debounce
        let changed = pub_.check_status("pane-1", ProcessStatus::Unknown, None, "Error occurred!");
        assert!(changed);
        assert_eq!(pub_.current_status("pane-1"), AgentStatus::Error);
    }

    #[test]
    fn test_check_status_exited_bypasses_debounce() {
        let bus = Arc::new(EventBus::new(8));
        let pub_ = StatusPublisher::new(Arc::clone(&bus));
        pub_.register_pane("pane-1");

        // Set to Running first
        pub_.check_status("pane-1", ProcessStatus::Running, None, "AI is thinking");
        pub_.check_status("pane-1", ProcessStatus::Running, None, "AI is thinking");
        assert_eq!(pub_.current_status("pane-1"), AgentStatus::Running);

        // Exited should bypass debounce
        let changed = pub_.check_status("pane-1", ProcessStatus::Exited, None, "any content");
        assert!(changed);
        assert_eq!(pub_.current_status("pane-1"), AgentStatus::Exited);
    }

    #[test]
    fn test_no_polling_thread() {
        // This test verifies the new implementation has no polling thread
        let bus = Arc::new(EventBus::new(8));
        let pub_ = StatusPublisher::new(bus);
        pub_.register_pane("test");
        pub_.check_status("test", ProcessStatus::Running, None, "content");
    }
}
