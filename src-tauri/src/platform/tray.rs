use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

const MAIN_WINDOW_LABEL: &str = "main";

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let open_item = MenuItem::with_id(app, "open", "Open Pi-Hub", true, None::<&str>)?;
    let refresh_all_item =
        MenuItem::with_id(app, "refresh_all", "Refresh All", true, None::<&str>)?;
    let exit_item = MenuItem::with_id(app, "exit", "Exit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[&open_item, &refresh_all_item, &separator, &exit_item],
    )?;

    let icon = app
        .default_window_icon()
        .cloned()
        .expect("bundle icon must be configured for the tray icon");

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("Pi-Hub")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main_window(app),
            "refresh_all" => {
                // Monitoring is implemented in a later work unit; this is a
                // menu-shape placeholder only.
                println!("Refresh All requested from tray (not yet implemented)");
            }
            "exit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { .. } = event {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
