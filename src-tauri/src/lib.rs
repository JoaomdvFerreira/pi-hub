mod application;
mod commands;
mod domain;
mod error;
mod infrastructure;
mod monitoring;
mod platform;
mod state;
mod storage;

use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be registered before other plugins/setup logic: if a second
        // instance is launched (e.g. a manual double-click while Pi-Hub is
        // already running in the tray), this callback runs in the *first*
        // instance and the second process exits immediately, so there's
        // never a second pi-hub.exe -- and therefore never a second tray
        // icon, a second scheduler, or doubled resource usage.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            platform::tray::show_main_window(app);
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("pihub".into()),
                    }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                ])
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized".into()]),
        ))
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_app_settings,
            commands::settings::save_app_settings,
            commands::devices::get_devices,
            commands::devices::get_device,
            commands::devices::create_device,
            commands::devices::update_device,
            commands::devices::delete_device,
            commands::devices::test_device_connection,
            commands::devices::open_device_service,
            commands::launch::open_device_terminal,
            commands::monitoring::refresh_device,
            commands::monitoring::refresh_all_devices,
            commands::monitoring::get_latest_snapshot,
            commands::containers::perform_container_action,
            commands::terminal::open_terminal_session,
            commands::terminal::write_terminal_input,
            commands::terminal::resize_terminal_session,
            commands::terminal::close_terminal_session
        ])
        .manage(monitoring::concurrency::RefreshCoordinator::new(
            monitoring::scheduler::MAX_CONCURRENT_REFRESHES,
        ))
        .manage(platform::pty::PtySessionManager::default())
        .setup(|app| {
            log::info!("Pi-Hub starting up");

            // Registered before anything else touches the window: WebView2
            // only delivers NavigationCompleted once the OS message loop
            // starts pumping (i.e. once .run() gets going, after setup()
            // returns), so attaching the handler here reliably catches
            // even the very first navigation -- including a startup
            // failure, which is exactly the case this exists to catch.
            #[cfg(windows)]
            platform::webview_fallback::install(app.handle());

            platform::tray::build(app.handle())?;
            monitoring::scheduler::start(app.handle().clone());

            // The main window is created hidden (tauri.conf.json's
            // `visible: false`) specifically so there is nothing to hide:
            // a normal launch shows it once, right here; an autostart
            // launch (`--minimized`, registered below) never calls show()
            // at all, so it settles straight into the tray with no
            // flash -- a create-visible-then-hide sequence would have
            // painted at least one frame of the window first.
            if !std::env::args().any(|arg| arg == "--minimized") {
                platform::tray::show_main_window(app.handle());
            }

            // Desktop platforms (Windows in particular, our only target)
            // grant this immediately with no user prompt; requesting it
            // up front is still the cross-platform-correct thing to do,
            // since other platforms this plugin supports do prompt.
            if let Err(err) =
                tauri_plugin_notification::NotificationExt::notification(app).request_permission()
            {
                log::warn!("could not request notification permission: {err}");
            }

            // Re-assert the OS-level autostart registration against the
            // persisted preference on every launch, not just when the
            // setting is saved -- keeps the two in sync even if something
            // external (a reinstall, a restored config.json, a user
            // manually clearing it from Windows' own Startup Apps page)
            // ever made them drift.
            if let Ok(config_dir) = app.path().app_config_dir() {
                use storage::config_repository::SettingsRepository;
                let settings =
                    storage::config_repository::JsonSettingsRepository::new(config_dir).load();
                if let Err(err) =
                    platform::autostart::sync(app.handle(), settings.start_with_windows)
                {
                    log::warn!("could not sync the Windows startup registration: {}", err.0);
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
