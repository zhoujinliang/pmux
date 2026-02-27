// main.rs - pmux GUI application using gpui
use gpui::{actions, *};
use pmux::ui::app_root::AppRoot;

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        // Register menu actions
        cx.on_action(open_settings);
        cx.on_action(select_workspace_from_menu);
        cx.on_action(toggle_sidebar_from_menu);
        cx.on_action(open_help);

        // Set up macOS-style application menus
        cx.set_menus(vec![
            Menu {
                name: "Settings".into(),
                items: vec![MenuItem::action("Preferences…", OpenSettings)],
            },
            Menu {
                name: "File".into(),
                items: vec![MenuItem::action("Select Workspace…", SelectWorkspaceFromMenu)],
            },
            Menu {
                name: "Edit".into(),
                items: vec![],
            },
            Menu {
                name: "View".into(),
                items: vec![MenuItem::action("Toggle Sidebar", ToggleSidebarFromMenu)],
            },
            Menu {
                name: "Help".into(),
                items: vec![MenuItem::action("pmux Help", OpenHelp)],
            },
        ]);

        let bounds = Bounds::centered(None, size(px(900.), px(600.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                // Register window close handler to quit the application
                // when the main window is closed.
                window.on_window_should_close(cx, |_window, app| {
                    app.quit();
                    true // Allow the window to close
                });

                cx.new(|cx| {
                    let mut app_root = AppRoot::new();
                    app_root.init_workspace_restoration(cx);
                    app_root
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}

// Application-wide menu actions (currently placeholders)
actions!(pmux_menus, [
    OpenSettings,
    SelectWorkspaceFromMenu,
    ToggleSidebarFromMenu,
    OpenHelp,
]);

fn open_settings(_: &OpenSettings, _cx: &mut App) {
    println!("Settings menu clicked - TODO: open settings UI");
}

fn select_workspace_from_menu(_: &SelectWorkspaceFromMenu, _cx: &mut App) {
    println!("File > Select Workspace… clicked - TODO: trigger workspace picker");
}

fn toggle_sidebar_from_menu(_: &ToggleSidebarFromMenu, _cx: &mut App) {
    println!("View > Toggle Sidebar clicked - TODO: toggle sidebar visibility");
}

fn open_help(_: &OpenHelp, _cx: &mut App) {
    println!("Help > pmux Help clicked - TODO: open documentation");
}
