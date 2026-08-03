use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Wry,
};

use crate::domain::device::Device;
use crate::monitoring::scheduler;
use crate::platform::pty::PtySessionManager;
use crate::platform::terminal;
use crate::storage::device_repository::{DeviceRepository, JsonDeviceRepository};

const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_ID: &str = "main-tray";

const TERMINAL_ID_PREFIX: &str = "terminal:";
const SERVICE_ID_PREFIX: &str = "service:";

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;

    let icon = app
        .default_window_icon()
        .cloned()
        .expect("bundle icon must be configured for the tray icon");

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("Pi-Hub")
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { .. } = event {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// Rebuilds and re-applies the tray menu so its per-device shortcuts stay
/// in sync after a device is created, edited, or deleted. Called from the
/// device CRUD commands rather than kept in sync incrementally, since the
/// menu is small and rebuilding it is cheap.
pub fn rebuild_device_menu(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    match build_menu(app) {
        Ok(menu) => {
            if let Err(err) = tray.set_menu(Some(menu)) {
                log::warn!("failed to rebuild the tray menu: {err}");
            }
        }
        Err(err) => log::warn!("failed to build the tray menu: {err}"),
    }
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let open_item = MenuItem::with_id(app, "open", "Open Pi-Hub", true, None::<&str>)?;
    let refresh_all_item =
        MenuItem::with_id(app, "refresh_all", "Refresh All", true, None::<&str>)?;
    let devices_submenu = build_devices_submenu(app)?;
    let exit_item = MenuItem::with_id(app, "exit", "Exit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;

    Menu::with_items(
        app,
        &[
            &open_item,
            &refresh_all_item,
            &separator,
            &devices_submenu,
            &separator,
            &exit_item,
        ],
    )
}

fn build_devices_submenu(app: &AppHandle) -> tauri::Result<Submenu<Wry>> {
    let devices = load_devices(app);

    if devices.is_empty() {
        let placeholder = MenuItem::with_id(
            app,
            "no_devices",
            "No devices registered",
            false,
            None::<&str>,
        )?;
        return Submenu::with_items(app, "Devices", true, &[&placeholder]);
    }

    let mut device_items: Vec<Submenu<Wry>> = Vec::with_capacity(devices.len());
    for device in &devices {
        device_items.push(build_device_submenu(app, device)?);
    }
    let refs: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = device_items
        .iter()
        .map(|s| s as &dyn tauri::menu::IsMenuItem<Wry>)
        .collect();
    Submenu::with_items(app, "Devices", true, &refs)
}

fn build_device_submenu(app: &AppHandle, device: &Device) -> tauri::Result<Submenu<Wry>> {
    let terminal_item = MenuItem::with_id(
        app,
        format!("{TERMINAL_ID_PREFIX}{}", device.id),
        "Open Terminal",
        true,
        None::<&str>,
    )?;

    let enabled_services: Vec<_> = device.services.iter().filter(|s| s.enabled).collect();
    if enabled_services.is_empty() {
        return Submenu::with_items(app, &device.name, true, &[&terminal_item]);
    }

    let separator = PredefinedMenuItem::separator(app)?;
    let mut service_items = Vec::with_capacity(enabled_services.len());
    for service in &enabled_services {
        service_items.push(MenuItem::with_id(
            app,
            format!("{SERVICE_ID_PREFIX}{}:{}", device.id, service.id),
            format!("Open {}", service.name),
            true,
            None::<&str>,
        )?);
    }

    let mut items: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = vec![&terminal_item, &separator];
    for item in &service_items {
        items.push(item);
    }
    Submenu::with_items(app, &device.name, true, &items)
}

fn load_devices(app: &AppHandle) -> Vec<Device> {
    let Ok(config_dir) = app.path().app_config_dir() else {
        return Vec::new();
    };
    JsonDeviceRepository::new(config_dir).load_all()
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    let id = event.id().as_ref();
    match id {
        "open" => show_main_window(app),
        "refresh_all" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = scheduler::refresh_all(&app).await {
                    log::warn!("tray-triggered Refresh All failed: {}", err.message);
                }
            });
        }
        "exit" => exit_app(app),
        "no_devices" => {}
        _ if id.starts_with(TERMINAL_ID_PREFIX) => {
            let device_id = &id[TERMINAL_ID_PREFIX.len()..];
            open_terminal_for(app, device_id);
        }
        _ if id.starts_with(SERVICE_ID_PREFIX) => {
            let rest = &id[SERVICE_ID_PREFIX.len()..];
            if let Some((device_id, service_id)) = rest.split_once(':') {
                open_service_for(app, device_id, service_id);
            }
        }
        _ => {}
    }
}

fn open_terminal_for(app: &AppHandle, device_id: &str) {
    let devices = load_devices(app);
    let Some(device) = devices.into_iter().find(|d| d.id == device_id) else {
        return;
    };
    if let Err(err) =
        terminal::launch_ssh_terminal(&device.host, device.ssh_port, &device.ssh_username)
    {
        log::warn!("failed to launch terminal from the tray: {}", err.0);
    }
}

fn open_service_for(app: &AppHandle, device_id: &str, service_id: &str) {
    let devices = load_devices(app);
    let Some(device) = devices.into_iter().find(|d| d.id == device_id) else {
        return;
    };
    let Some(service) = device.services.into_iter().find(|s| s.id == service_id) else {
        return;
    };
    if let Err(err) = tauri_plugin_opener::open_url(&service.url, None::<&str>) {
        log::warn!("failed to open service from the tray: {err}");
    }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Explicit, deliberate shutdown -- as opposed to `app.exit(0)` on its own,
/// which relies on the runtime's own (usually-but-not-guaranteed graceful)
/// teardown to release everything. Two things specifically need to happen
/// *before* the process goes away, not "eventually" as a side effect of
/// exiting:
/// 1. Every live terminal session's ssh.exe must be killed here, while the
///    app can still act -- once the process is gone, nothing can ask a
///    child process spawned by it to close, and it survives as an orphan
///    exactly like the hang/leak bug this whole module exists to avoid,
///    just reached by quitting instead of a stuck session.
/// 2. The tray icon is removed explicitly via `remove_tray_by_id` rather
///    than left to whatever the runtime's shutdown path does with it --
///    Windows only reliably retires a tray icon when the owning process
///    calls Shell_NotifyIcon(NIM_DELETE) (which dropping the `TrayIcon`
///    here triggers) or when the shell itself later notices the process
///    is gone (which is what produces the "ghost icon that disappears
///    once you open the tray" most users have seen from *some* app at one
///    point). Doing it here guarantees the icon is gone immediately for
///    every deliberate exit through this menu, not just eventually.
fn exit_app(app: &AppHandle) {
    app.state::<PtySessionManager>().close_all();
    let _ = app.remove_tray_by_id(TRAY_ID);
    app.exit(0);
}
