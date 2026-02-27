// agent_status_test.rs - TDD tests for AgentStatus
#[cfg(test)]
mod tests {
    /// Test: All status variants exist
    #[test]
    fn test_status_variants() {
        let _running = AgentStatus::Running;
        let _waiting = AgentStatus::Waiting;
        let _idle = AgentStatus::Idle;
        let _error = AgentStatus::Error;
        let _unknown = AgentStatus::Unknown;
    }

    /// Test: Status color mapping
    #[test]
    fn test_status_colors() {
        assert_eq!(AgentStatus::Running.color(), "green");
        assert_eq!(AgentStatus::Waiting.color(), "yellow");
        assert_eq!(AgentStatus::Idle.color(), "gray");
        assert_eq!(AgentStatus::Error.color(), "red");
        assert_eq!(AgentStatus::Unknown.color(), "purple");
    }

    /// Test: Status icon mapping
    #[test]
    fn test_status_icons() {
        assert_eq!(AgentStatus::Running.icon(), "●");
        assert_eq!(AgentStatus::Waiting.icon(), "◐");
        assert_eq!(AgentStatus::Idle.icon(), "○");
        assert_eq!(AgentStatus::Error.icon(), "✕");
        assert_eq!(AgentStatus::Unknown.icon(), "?");
    }

    /// Test: Status display text
    #[test]
    fn test_status_display_text() {
        assert_eq!(AgentStatus::Running.display_text(), "Running");
        assert_eq!(AgentStatus::Waiting.display_text(), "Waiting for input");
        assert_eq!(AgentStatus::Idle.display_text(), "Idle");
        assert_eq!(AgentStatus::Error.display_text(), "Error detected");
        assert_eq!(AgentStatus::Unknown.display_text(), "Unknown");
    }

    /// Test: Status priority ordering
    #[test]
    fn test_status_priority() {
        // Error has highest priority
        assert!(AgentStatus::Error.priority() > AgentStatus::Waiting.priority());
        assert!(AgentStatus::Error.priority() > AgentStatus::Running.priority());
        
        // Waiting > Running > Idle > Unknown
        assert!(AgentStatus::Waiting.priority() > AgentStatus::Running.priority());
        assert!(AgentStatus::Running.priority() > AgentStatus::Idle.priority());
        assert!(AgentStatus::Idle.priority() > AgentStatus::Unknown.priority());
    }
}

// Placeholder enum for testing
enum AgentStatus {
    Running,
    Waiting,
    Idle,
    Error,
    Unknown,
}

impl AgentStatus {
    fn color(&self) -> &'static str {
        match self {
            AgentStatus::Running => "green",
            AgentStatus::Waiting => "yellow",
            AgentStatus::Idle => "gray",
            AgentStatus::Error => "red",
            AgentStatus::Unknown => "purple",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            AgentStatus::Running => "●",
            AgentStatus::Waiting => "◐",
            AgentStatus::Idle => "○",
            AgentStatus::Error => "✕",
            AgentStatus::Unknown => "?",
        }
    }

    fn display_text(&self) -> &'static str {
        match self {
            AgentStatus::Running => "Running",
            AgentStatus::Waiting => "Waiting for input",
            AgentStatus::Idle => "Idle",
            AgentStatus::Error => "Error detected",
            AgentStatus::Unknown => "Unknown",
        }
    }

    fn priority(&self) -> u8 {
        match self {
            AgentStatus::Error => 5,
            AgentStatus::Waiting => 4,
            AgentStatus::Running => 3,
            AgentStatus::Idle => 2,
            AgentStatus::Unknown => 1,
        }
    }
}
