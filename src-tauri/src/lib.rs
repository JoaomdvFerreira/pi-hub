mod application;
mod commands;
mod domain;
mod error;
mod infrastructure;
mod monitoring;
mod platform;
mod state;
mod storage;

use tauri::WindowEvent;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
            commands::monitoring::get_latest_snapshot
        ])
        .manage(monitoring::concurrency::RefreshCoordinator::new(
            monitoring::scheduler::MAX_CONCURRENT_REFRESHES,
        ))
        .setup(|app| {
            log::info!("Pi-Hub starting up");
            platform::tray::build(app.handle())?;
            monitoring::scheduler::start(app.handle().clone());
            // Desktop platforms (Windows in particular, our only target)
            // grant this immediately with no user prompt; requesting it
            // up front is still the cross-platform-correct thing to do,
            // since other platforms this plugin supports do prompt.
            if let Err(err) = tauri_plugin_notification::NotificationExt::notification(app)
                .request_permission()
            {
                log::warn!("could not request notification permission: {err}");
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
