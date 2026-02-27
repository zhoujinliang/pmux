// workspace_manager_test.rs - TDD tests for workspace management
#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    /// Test: WorkspaceTab creation
    #[test]
    fn test_workspace_tab_creation() {
        let tab = WorkspaceTab::new(
            PathBuf::from("/home/user/project1"),
            "project1"
        );
        
        assert_eq!(tab.name(), "project1");
        assert_eq!(tab.path(), &PathBuf::from("/home/user/project1"));
        assert!(!tab.is_modified());
    }

    /// Test: WorkspaceManager creation
    #[test]
    fn test_workspace_manager_creation() {
        let manager = WorkspaceManager::new();
        assert_eq!(manager.tab_count(), 0);
        assert!(manager.active_tab().is_none());
    }

    /// Test: Add workspace tab
    #[test]
    fn test_add_workspace_tab() {
        let mut manager = WorkspaceManager::new();
        let path = PathBuf::from("/tmp/project1");
        
        let index = manager.add_workspace(path.clone());
        
        assert_eq!(manager.tab_count(), 1);
        assert_eq!(manager.active_tab_index(), Some(0));
        assert_eq!(manager.get_tab(index).unwrap().path(), &path);
    }

    /// Test: Switch active tab
    #[test]
    fn test_switch_active_tab() {
        let mut manager = WorkspaceManager::new();
        manager.add_workspace(PathBuf::from("/tmp/project1"));
        manager.add_workspace(PathBuf::from("/tmp/project2"));
        
        manager.switch_to_tab(1);
        
        assert_eq!(manager.active_tab_index(), Some(1));
    }

    /// Test: Close tab
    #[test]
    fn test_close_tab() {
        let mut manager = WorkspaceManager::new();
        manager.add_workspace(PathBuf::from("/tmp/project1"));
        manager.add_workspace(PathBuf::from("/tmp/project2"));
        
        manager.close_tab(0);
        
        assert_eq!(manager.tab_count(), 1);
        assert_eq!(manager.active_tab_index(), Some(0));
    }

    /// Test: Tab name collision handling
    #[test]
    fn test_tab_name_uniqueness() {
        let mut manager = WorkspaceManager::new();
        manager.add_workspace(PathBuf::from("/tmp/project1"));
        manager.add_workspace(PathBuf::from("/home/user/project1"));
        
        // Should have unique display names
        let names: Vec<_> = manager.tabs()
            .map(|t| t.display_name().to_string())
            .collect();
        
        assert_ne!(names[0], names[1]);
    }
}

// Placeholder structs for testing
struct WorkspaceTab {
    path: PathBuf,
    name: String,
}

impl WorkspaceTab {
    fn new(path: PathBuf, name: &str) -> Self {
        Self { path, name: name.to_string() }
    }
    fn name(&self) -> &str { &self.name }
    fn path(&self) -> &PathBuf { &self.path }
    fn is_modified(&self) -> bool { false }
    fn display_name(&self) -> &str { &self.name }
}

struct WorkspaceManager {
    tabs: Vec<WorkspaceTab>,
    active_index: Option<usize>,
}

impl WorkspaceManager {
    fn new() -> Self {
        Self { tabs: Vec::new(), active_index: None }
    }
    fn tab_count(&self) -> usize { self.tabs.len() }
    fn active_tab(&self) -> Option<&WorkspaceTab> { 
        self.active_index.and_then(|i| self.tabs.get(i)) 
    }
    fn active_tab_index(&self) -> Option<usize> { self.active_index }
    fn add_workspace(&mut self, path: PathBuf) -> usize {
        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string());
        self.tabs.push(WorkspaceTab::new(path, &name));
        if self.active_index.is_none() {
            self.active_index = Some(0);
        }
        self.tabs.len() - 1
    }
    fn switch_to_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_index = Some(index);
        }
    }
    fn close_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.tabs.remove(index);
            if let Some(active) = self.active_index {
                if active >= self.tabs.len() && !self.tabs.is_empty() {
                    self.active_index = Some(self.tabs.len() - 1);
                } else if self.tabs.is_empty() {
                    self.active_index = None;
                }
            }
        }
    }
    fn get_tab(&self, index: usize) -> Option<&WorkspaceTab> {
        self.tabs.get(index)
    }
    fn tabs(&self) -> impl Iterator<Item = &WorkspaceTab> {
        self.tabs.iter()
    }
}
