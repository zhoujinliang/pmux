use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Snapshot of terminal content for a pane
#[derive(Clone, Default)]
pub struct PaneSnapshot {
    pub pane_id: String,
    pub content: String,
    pub content_hash: u64,
}

impl PaneSnapshot {
    pub fn new(pane_id: &str, content: String) -> Self {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let hash = hasher.finish();
        Self {
            pane_id: pane_id.to_string(),
            content,
            content_hash: hash,
        }
    }

    pub fn has_changed(&self, other: &PaneSnapshot) -> bool {
        self.content_hash != other.content_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_hash_same_content() {
        let a = PaneSnapshot::new("%0", "hello world".to_string());
        let b = PaneSnapshot::new("%0", "hello world".to_string());
        assert!(!a.has_changed(&b));
    }

    #[test]
    fn test_snapshot_hash_different_content() {
        let a = PaneSnapshot::new("%0", "hello".to_string());
        let b = PaneSnapshot::new("%0", "world".to_string());
        assert!(a.has_changed(&b));
    }

    #[test]
    fn test_snapshot_empty() {
        let a = PaneSnapshot::new("%0", "".to_string());
        let b = PaneSnapshot::new("%0", "".to_string());
        assert!(!a.has_changed(&b));
    }
}
