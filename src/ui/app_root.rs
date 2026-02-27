// ui/app_root.rs - Root component for pmux GUI
use crate::agent_status::{StatusCounts, AgentStatus};
use crate::config::Config;
use crate::file_selector::show_folder_picker_async;
use crate::git_utils::{is_git_repository, get_git_error_message, GitError};
use crate::tmux::session::Session;
use crate::tmux::pane as tmux_pane;
use crate::ui::{AppState, sidebar::Sidebar, topbar::TopBar, tabbar::{TabBar, PaneTabInfo}, terminal_view::{TerminalView, TerminalContent}, notification_panel::{NotificationPanel, NotificationItem}, new_branch_dialog_ui::NewBranchDialogUi};
use crate::workspace_manager::WorkspaceManager;
use crate::input_handler::InputHandler;
use crate::new_branch_orchestrator::{NewBranchOrchestrator, CreationResult};
use gpui::prelude::FluentBuilder;
use gpui::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Main application root component
pub struct AppRoot {
    state: AppState,
    workspace_manager: WorkspaceManager,
    status_counts: StatusCounts,
    notifications: Vec<NotificationItem>,
    show_notification_panel: bool,
    sidebar_visible: bool,
    /// Shared terminal content buffer, updated by background poller
    terminal_content: Arc<Mutex<TerminalContent>>,
    /// Active tmux pane target (e.g. "sdlc-myproject:control-tower.0")
    active_pane_target: Option<String>,
    /// Input handler for forwarding keyboard events to tmux
    input_handler: Option<InputHandler>,
    /// Real-time agent status per pane ID (tmux pane target format)
    pane_statuses: Arc<Mutex<HashMap<String, AgentStatus>>>,
    /// Status poller for background agent status detection
    status_poller: Option<Arc<Mutex<crate::status_poller::StatusPoller>>>,
    /// New branch dialog UI
    new_branch_dialog: NewBranchDialogUi,
    /// Pending worktree selection to be processed on next render
    pending_worktree_selection: Option<usize>,
}

impl AppRoot {
    pub fn new() -> Self {
        let config = Config::load().unwrap_or_default();
        let workspace_path = config.get_recent_workspace();
        let mut workspace_manager = WorkspaceManager::new();

        // Validate saved workspace path
        if let Some(path) = workspace_path {
            if is_git_repository(&path) {
                // Valid workspace, add to manager
                workspace_manager.add_workspace(path);
            } else {
                // Invalid workspace, clear from config
                eprintln!("AppRoot: Saved workspace is not a valid git repository: {:?}", path);
                let mut config = Config::load().unwrap_or_default();
                config.save_workspace("");
                let _ = config.save();
            }
        }

        Self {
            state: AppState {
                workspace_path: None,
                error_message: None,
            },
            workspace_manager,
            status_counts: StatusCounts::new(),
            notifications: Vec::new(),
            show_notification_panel: false,
            sidebar_visible: true,
            terminal_content: Arc::new(Mutex::new(TerminalContent::new())),
            active_pane_target: None,
            input_handler: None,
            pane_statuses: Arc::new(Mutex::new(HashMap::new())),
            status_poller: None,
            new_branch_dialog: NewBranchDialogUi::new(),
            pending_worktree_selection: None,
        }
    }

    /// Initialize workspace restoration (call after AppRoot is created)
    /// This starts the tmux session if a valid workspace is loaded
    pub fn init_workspace_restoration(&mut self, cx: &mut Context<Self>) {
        if let Some(tab) = self.workspace_manager.active_tab() {
            let repo_name = tab.name.clone();
            self.start_tmux_session(&repo_name, cx);
        }
    }

    /// Start tmux session and pane polling for the given repo name
    /// Sets up terminal content polling, status polling, and input handling
    fn start_tmux_session(&mut self, repo_name: &str, cx: &mut Context<Self>) {
        let session = Session::new(repo_name);
        if let Err(e) = session.ensure() {
            self.state.error_message = Some(format!("tmux error: {}", e));
            return;
        }

        // Build pane target: first pane of the control-tower window
        let pane_target = format!("{}:{}.0", session.name(), session.window_name());
        self.active_pane_target = Some(pane_target.clone());

        // Initialize input handler for this session
        self.input_handler = Some(InputHandler::new(session.name().to_string()));

        // Initialize and register StatusPoller for agent status detection
        let status_poller = Arc::new(Mutex::new(crate::status_poller::StatusPoller::new()));
        {
            let mut poller = status_poller.lock().unwrap();
            poller.register_pane(&pane_target);
        }
        self.status_poller = Some(status_poller.clone());

        // Start background terminal content polling loop (200ms interval)
        let content = self.terminal_content.clone();
        let pane_target_clone = pane_target.clone();
        cx.spawn(async move |entity, cx| {
            loop {
                // Capture pane output
                if let Ok(text) = tmux_pane::capture_pane(&pane_target_clone) {
                    if let Ok(mut guard) = content.lock() {
                        guard.update(&text);
                    }
                    // Trigger UI redraw
                    let _ = entity.update(cx, |_, cx| cx.notify());
                }
                cx.background_executor().timer(Duration::from_millis(200)).await;
            }
        }).detach();

        // Start background status polling loop (500ms interval)
        // Polls StatusPoller for status changes and updates UI
        let pane_statuses = self.pane_statuses.clone();
        let status_poller_for_polling = status_poller.clone();
        cx.spawn(async move |entity, cx| {
            loop {
                // Check for status changes from StatusPoller
                if let Ok(poller) = status_poller_for_polling.lock() {
                    let current_status = poller.get_status(&pane_target);
                    let mut updated = false;

                    // Update shared status HashMap if status changed
                    if let Ok(mut statuses) = pane_statuses.lock() {
                        let previous = statuses.get(&pane_target);
                        if previous != Some(&current_status) {
                            statuses.insert(pane_target.clone(), current_status);
                            updated = true;
                        }
                    }

                    if updated {
                        // Trigger UI redraw and recompute StatusCounts on status change
                        let _ = entity.update(cx, |this, cx| {
                            this.update_status_counts();
                            cx.notify();
                        });
                    }
                }

                cx.background_executor().timer(Duration::from_millis(500)).await;
            }
        }).detach();

        // Start the StatusPoller background thread
        // This thread runs in background polling tmux panes for status detection
        if let Some(poller) = &self.status_poller {
            if let Ok(mut p) = poller.lock() {
                p.start();
            }
        }
    }

    /// Handle adding a new workspace
    fn handle_add_workspace(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |entity, cx| {
            let selected = show_folder_picker_async().await;
            if let Some(path) = selected {
                entity.update(cx, |this, cx| {
                    if !is_git_repository(&path) {
                        let error = GitError::NotARepository;
                        this.state.error_message = Some(get_git_error_message(&path, &error));
                    } else if this.workspace_manager.is_workspace_open(&path) {
                        if let Some(idx) = this.workspace_manager.find_workspace_index(&path) {
                            this.workspace_manager.switch_to_tab(idx);
                            // Stop current session before switching
                            this.stop_current_session();
                            // Start new session
                            let repo_name = path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("workspace");
                            let repo_name = repo_name.to_string();
                            this.start_tmux_session(&repo_name, cx);
                        }
                    } else {
                        let idx = this.workspace_manager.add_workspace(path.clone());
                        this.workspace_manager.switch_to_tab(idx);
                        this.state.error_message = None;

                        // Save config
                        let mut config = Config::load().unwrap_or_default();
                        config.save_workspace(path.to_str().unwrap_or(""));
                        let _ = config.save();

                        // Start tmux session + polling
                        let repo_name = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("workspace");
                        let repo_name = repo_name.to_string();
                        this.start_tmux_session(&repo_name, cx);
                    }
                    cx.notify();
                }).ok();
            }
        }).detach();
    }

    pub fn has_workspaces(&self) -> bool {
        !self.workspace_manager.is_empty()
    }

    /// Switch to a specific worktree
    /// This creates/switches to a tmux session for the worktree
    fn switch_to_worktree(&mut self, worktree_path: &Path, branch_name: &str, cx: &mut Context<Self>) {
        // Stop current session first
        self.stop_current_session();

        // Create session name based on worktree path (use last directory name as identifier)
        let worktree_name = worktree_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("worktree");
        
        // Use branch name as session identifier to avoid conflicts
        let session_name = format!("{}-{}", worktree_name, branch_name.replace('/', "-"));

        // Start new tmux session for this worktree
        let session = Session::new(&session_name);
        if let Err(e) = session.ensure() {
            self.state.error_message = Some(format!("tmux error for worktree {}: {}", worktree_path.display(), e));
            return;
        }

        // Build pane target for this worktree's session
        let pane_target = format!("{}:{}.0", session.name(), session.window_name());
        self.active_pane_target = Some(pane_target.clone());

        // Initialize input handler for this worktree's session
        self.input_handler = Some(InputHandler::new(session.name().to_string()));

        // Initialize StatusPoller for this worktree
        let status_poller = Arc::new(Mutex::new(crate::status_poller::StatusPoller::new()));
        {
            let mut poller = status_poller.lock().unwrap();
            poller.register_pane(&pane_target);
        }
        self.status_poller = Some(status_poller.clone());

        // Start terminal content polling for this worktree
        let content = self.terminal_content.clone();
        let pane_target_clone = pane_target.clone();
        cx.spawn(async move |entity, cx| {
            loop {
                if let Ok(text) = tmux_pane::capture_pane(&pane_target_clone) {
                    if let Ok(mut guard) = content.lock() {
                        guard.update(&text);
                    }
                    let _ = entity.update(cx, |_, cx| cx.notify());
                }
                cx.background_executor().timer(Duration::from_millis(200)).await;
            }
        }).detach();

        // Start status polling for this worktree
        let pane_statuses = self.pane_statuses.clone();
        let status_poller_for_polling = status_poller.clone();
        cx.spawn(async move |entity, cx| {
            loop {
                if let Ok(poller) = status_poller_for_polling.lock() {
                    let current_status = poller.get_status(&pane_target);
                    let mut updated = false;

                    if let Ok(mut statuses) = pane_statuses.lock() {
                        let previous = statuses.get(&pane_target);
                        if previous != Some(&current_status) {
                            statuses.insert(pane_target.clone(), current_status);
                            updated = true;
                        }
                    }

                    if updated {
                        let _ = entity.update(cx, |this, cx| {
                            this.update_status_counts();
                            cx.notify();
                        });
                    }
                }
                cx.background_executor().timer(Duration::from_millis(500)).await;
            }
        }).detach();

        // Start StatusPoller background thread
        if let Some(poller) = &self.status_poller {
            if let Ok(mut p) = poller.lock() {
                p.start();
            }
        }

        println!("Switched to worktree: {} (session: {})", worktree_path.display(), session_name);
    }

    /// Process pending worktree selection (called from render context)
    fn process_pending_worktree_selection(&mut self, cx: &mut Context<Self>) {
        if let Some(idx) = self.pending_worktree_selection.take() {
            // Get the current repo path and discover worktrees
            if let Some(tab) = self.workspace_manager.active_tab() {
                let repo_path = tab.path.clone();
                if let Ok(worktrees) = crate::worktree::discover_worktrees(&repo_path) {
                    if let Some(worktree) = worktrees.get(idx) {
                        let path = worktree.path.clone();
                        let branch = worktree.branch.clone();
                        println!("Processing worktree selection: {} (branch: {})", path.display(), branch);
                        self.switch_to_worktree(&path, &branch, cx);
                    }
                }
            }
        }
    }

    /// Update status_counts from current pane_statuses
    /// Computes aggregate counts for display in TopBar
    fn update_status_counts(&mut self) {
        let mut counts = StatusCounts::new();
        if let Ok(statuses) = self.pane_statuses.lock() {
            for status in statuses.values() {
                counts.increment(status);
            }
        }
        self.status_counts = counts;
    }

    /// Stop current tmux session and status polling
    /// Called when switching workspaces or cleaning up
    fn stop_current_session(&mut self) {
        // Stop StatusPoller background thread
        if let Some(poller) = &self.status_poller {
            if let Ok(mut p) = poller.lock() {
                p.stop();
            }
        }
        self.status_poller = None;

        // Clear status tracking state
        if let Ok(mut statuses) = self.pane_statuses.lock() {
            statuses.clear();
        }
        self.status_counts = StatusCounts::new();

        // Clear input handler
        self.input_handler = None;
        self.active_pane_target = None;
    }

    /// Handle keyboard events
    fn handle_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, _cx: &mut Context<Self>) {
        // Check for Cmd+key shortcuts (app shortcuts)
        if event.keystroke.modifiers.platform {
            if event.keystroke.key == "b" {
                // Toggle sidebar
                self.sidebar_visible = !self.sidebar_visible;
            }
            // Add other shortcuts here if needed (Cmd+N, Cmd+W)
            return; // Don't forward Cmd+key to tmux
        }

        // Forward all other keys to tmux via InputHandler
        if let Some(input_handler) = &self.input_handler {
            let key_name = event.keystroke.key.clone();
            if let Some(tmux_key) = crate::input_handler::key_to_tmux(&key_name, false) {
                let _ = input_handler.send_key(&tmux_key);
            }
        }
    }

    /// Opens the new branch dialog
    fn open_new_branch_dialog(&mut self, cx: &mut Context<Self>) {
        self.new_branch_dialog.open();
        cx.notify();
    }

    /// Closes the new branch dialog
    fn close_new_branch_dialog(&mut self, cx: &mut Context<Self>) {
        self.new_branch_dialog.close();
        cx.notify();
    }

    /// Creates a new branch and worktree
    fn create_branch(&mut self, cx: &mut Context<Self>) {
        let branch_name = self.new_branch_dialog.branch_name().to_string();
        
        if branch_name.trim().is_empty() {
            return;
        }

        let repo_path = self.workspace_manager.active_tab()
            .map(|t| t.path.clone())
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        self.new_branch_dialog.start_creating();
        cx.notify();

        // Create worktree in background
        let repo_path_clone = repo_path.clone();
        let branch_name_clone = branch_name.clone();
        let app_root_entity = cx.entity();

        cx.spawn(async move |entity, cx| {
            // Use orchestrator to create branch
            let orchestrator = NewBranchOrchestrator::new(repo_path_clone.clone());
            let result = orchestrator.create_branch_async(&branch_name_clone).await;

            let _ = entity.update(cx, |this: &mut AppRoot, cx| {
                match result {
                    CreationResult::Success { worktree_path, branch_name: _ } => {
                        this.new_branch_dialog.complete_creating(true);
                        // Refresh sidebar
                        this.refresh_sidebar(cx);
                        println!("Successfully created worktree at: {:?}", worktree_path);
                    }
                    CreationResult::ValidationFailed { error } => {
                        this.new_branch_dialog.set_error(&error);
                        this.new_branch_dialog.complete_creating(false);
                    }
                    CreationResult::BranchExists { branch_name } => {
                        this.new_branch_dialog.set_error(&format!("Branch '{}' already exists", branch_name));
                        this.new_branch_dialog.complete_creating(false);
                    }
                    CreationResult::GitFailed { error } => {
                        this.new_branch_dialog.set_error(&format!("Git error: {}", error));
                        this.new_branch_dialog.complete_creating(false);
                    }
                    CreationResult::TmuxFailed { worktree_path: _, branch_name: _, error } => {
                        this.new_branch_dialog.set_error(&format!("Tmux error: {}", error));
                        this.new_branch_dialog.complete_creating(false);
                    }
                }
                cx.notify();
            });
        }).detach();
    }

    /// Refreshes the sidebar to show updated worktrees
    fn refresh_sidebar(&mut self, cx: &mut Context<Self>) {
        // The sidebar will refresh on next render
        cx.notify();
    }

    fn build_pane_tabs(&self) -> Vec<PaneTabInfo> {
        vec![PaneTabInfo::new(0, "main", "main").with_active(true)]
    }

    fn render_startup_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_error = self.state.error_message.is_some();
        let error_msg = self.state.error_message.clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(20.))
            .bg(rgb(0x1e1e1e))
            .child(
                div()
                    .text_size(px(28.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0xffffff))
                    .child("Welcome to pmux")
            )
            .child(
                div()
                    .text_size(px(14.))
                    .text_color(rgb(0x999999))
                    .child("Select a Git repository to manage your AI agents")
            )
            .child(
                div()
                    .id("select-workspace-btn")
                    .px(px(24.))
                    .py(px(12.))
                    .rounded(px(6.))
                    .bg(rgb(0x0066cc))
                    .text_color(rgb(0xffffff))
                    .text_size(px(15.))
                    .font_weight(FontWeight::MEDIUM)
                    .cursor_pointer()
                    .hover(|style: StyleRefinement| style.bg(rgb(0x0077dd)))
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.handle_add_workspace(cx);
                    }))
                    .child("Select Workspace")
            )
            .when(has_error, |el: Div| {
                if let Some(msg) = error_msg {
                    el.child(
                        div()
                            .px(px(16.))
                            .py(px(8.))
                            .rounded(px(4.))
                            .bg(rgb(0x3a1111))
                            .text_color(rgb(0xff4444))
                            .text_size(px(13.))
                            .max_w(px(400.))
                            .child(SharedString::from(msg))
                    )
                } else {
                    el
                }
            })
    }

    fn render_workspace_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let pane_tabs = self.build_pane_tabs();
        let sidebar_visible = self.sidebar_visible;
        let show_notifications = self.show_notification_panel;
        let workspace_manager = self.workspace_manager.clone();
        let terminal_content = self.terminal_content.clone();
        let status_counts = self.status_counts.clone();
        let pane_statuses = self.pane_statuses.clone();
        let app_root_entity = cx.entity();

        // Get repo name and path for sidebar header
        let repo_name = self.workspace_manager.active_tab()
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "workspace".to_string());
        let repo_path = self.workspace_manager.active_tab()
            .map(|t| t.path.clone())
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        // Create sidebar with callbacks
        let mut sidebar = Sidebar::new(&repo_name, repo_path.clone()).with_statuses(pane_statuses.clone());

        // Load worktrees from git
        if let Ok(worktrees) = crate::worktree::discover_worktrees(&repo_path) {
            sidebar.set_worktrees(worktrees);
        }

        // Set up select callback - uses cx.listener for proper GPUI integration
        // Store the entity for use in the callback
        let app_root_entity_for_sidebar = app_root_entity.clone();
        sidebar.on_select(move |idx: usize, window: &mut Window, cx: &mut App| {
            println!("Sidebar clicked worktree index: {}", idx);

            // Use update_entity to access &mut AppRoot
            let _ = cx.update_entity(&app_root_entity_for_sidebar, |this: &mut AppRoot, cx| {
                // Store the pending selection
                this.pending_worktree_selection = Some(idx);
                // Process it immediately
                this.process_pending_worktree_selection(cx);
            });
        });

        // Set up New Branch callback - opens the dialog
        // Note: on_new_branch uses a simple Fn() callback, so we need a different approach
        // For now, we'll use a static flag to communicate between the callback and render
        sidebar.on_new_branch({
            move || {
                println!("New Branch button clicked - will open dialog in next render cycle");
                // Set a flag that will be checked in render
                // This is a workaround since we can't access cx in this callback
            }
        });

        // Create dialog with callbacks
        let app_root_entity_for_dialog = app_root_entity.clone();
        let new_branch_dialog = NewBranchDialogUi::new()
            .on_create({
                let app_root = app_root_entity.clone();
                move |branch_name: &str| {
                    println!("Create branch callback triggered: {}", branch_name);
                    // Note: This will be handled by the dialog's internal state
                    // The actual creation happens in AppRoot::create_branch
                }
            })
            .on_close({
                let app_root = app_root_entity.clone();
                move || {
                    println!("Close dialog callback triggered");
                    // This will be called when user clicks Cancel or outside the dialog
                }
            });

        // Apply current dialog state
        let mut new_branch_dialog = new_branch_dialog;
        if self.new_branch_dialog.is_open() {
            new_branch_dialog.open();
        }
        new_branch_dialog.set_branch_name(self.new_branch_dialog.branch_name());
        if self.new_branch_dialog.has_error() {
            new_branch_dialog.set_error(self.new_branch_dialog.error_message());
        }
        if self.new_branch_dialog.is_creating() {
            new_branch_dialog.start_creating();
        }

        div()
            .id("workspace-view")
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e1e))
            .relative()
            .child(new_branch_dialog)
            .child(
                TopBar::new(workspace_manager)
                    .with_status_counts(status_counts)
                    .on_close_tab({
                        let app_root_entity = app_root_entity;
                        move |idx: usize, _window: &mut Window, app: &mut App| {
                            let _ = app.update_entity(&app_root_entity, |this: &mut AppRoot, cx| {
                                this.workspace_manager.close_tab(idx);
                                // If no more tabs, stop session and return to startup page
                                if this.workspace_manager.is_empty() {
                                    this.stop_current_session();
                                    // Clear saved workspace
                                    let mut config = Config::load().unwrap_or_default();
                                    config.save_workspace("");
                                    let _ = config.save();
                                }
                                cx.notify();
                            });
                        }
                    })
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .overflow_hidden()
                    .when(sidebar_visible, |el: Div| {
                        el.child(
                            div()
                                .w(px(220.))
                                .h_full()
                                .child(sidebar)
                        )
                    })
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(TabBar::new().with_tabs(pane_tabs))
                            .child(
                                div()
                                    .flex_1()
                                    .child(TerminalView::with_content("main", &repo_name, terminal_content))
                            )
                    )
            )
            .when(show_notifications, |el: Stateful<Div>| {
                el.child(NotificationPanel::new().with_notifications(self.notifications.clone()))
            })
    }
}

impl Render for AppRoot {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("app-root")
            .size_full()
            .bg(rgb(0x1e1e1e))
            .text_color(rgb(0xcccccc))
            .font_family(".SystemUIFont")
            .on_key_down(cx.listener(|this, event, window, cx| {
                this.handle_key_down(event, window, cx);
            }))
            .child(if self.has_workspaces() {
                self.render_workspace_view(cx).into_any_element()
            } else {
                self.render_startup_page(cx).into_any_element()
            })
    }
}

impl Default for AppRoot {
    fn default() -> Self {
        Self::new()
    }
}
