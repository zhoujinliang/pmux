// notification_test.rs - TDD tests for notification system
#[cfg(test)]
mod tests {
    use std::time::Instant;

    /// Test: Notification creation
    #[test]
    fn test_notification_creation() {
        let notif = Notification::new(
            "pane-1",
            NotificationType::Error,
            "Build failed"
        );
        
        assert_eq!(notif.pane_id(), "pane-1");
        assert_eq!(notif.notif_type(), NotificationType::Error);
        assert_eq!(notif.message(), "Build failed");
        assert!(!notif.is_read());
    }

    /// Test: Notification type variants
    #[test]
    fn test_notification_types() {
        let _error = NotificationType::Error;
        let _waiting = NotificationType::Waiting;
        let _info = NotificationType::Info;
    }

    /// Test: Mark as read
    #[test]
    fn test_mark_as_read() {
        let mut notif = Notification::new("pane-1", NotificationType::Error, "Test");
        assert!(!notif.is_read());
        
        notif.mark_as_read();
        assert!(notif.is_read());
    }

    /// Test: Priority ordering
    #[test]
    fn test_priority_ordering() {
        // Error has highest priority
        assert!(NotificationType::Error.priority() > NotificationType::Waiting.priority());
        assert!(NotificationType::Waiting.priority() > NotificationType::Info.priority());
    }
}

// Placeholder structs for testing
enum NotificationType {
    Error,
    Waiting,
    Info,
}

impl NotificationType {
    fn priority(&self) -> u8 {
        match self {
            NotificationType::Error => 3,
            NotificationType::Waiting => 2,
            NotificationType::Info => 1,
        }
    }
}

struct Notification {
    pane_id: String,
    notif_type: NotificationType,
    message: String,
    read: bool,
    timestamp: Instant,
}

impl Notification {
    fn new(pane_id: &str, notif_type: NotificationType, message: &str) -> Self {
        Self {
            pane_id: pane_id.to_string(),
            notif_type,
            message: message.to_string(),
            read: false,
            timestamp: Instant::now(),
        }
    }

    fn pane_id(&self) -> &str { &self.pane_id }
    fn notif_type(&self) -> &NotificationType { &self.notif_type }
    fn message(&self) -> &str { &self.message }
    fn is_read(&self) -> bool { self.read }
    fn mark_as_read(&mut self) { self.read = true; }
}
