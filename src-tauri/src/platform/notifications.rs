use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::domain::notification_rule::NotificationEvent;
use crate::monitoring::notifications::NotificationService;

/// Dispatches notification-ready decisions (WU012's `evaluate_snapshot_
/// transition`) as native Windows toast notifications, via the Tauri
/// notification plugin. One toast per event, titled with the device name
/// so multiple devices' notifications stay distinguishable.
pub struct TauriNotificationService<'a> {
    app: &'a AppHandle,
    device_name: String,
}

impl<'a> TauriNotificationService<'a> {
    pub fn new(app: &'a AppHandle, device_name: impl Into<String>) -> Self {
        Self {
            app,
            device_name: device_name.into(),
        }
    }
}

impl NotificationService for TauriNotificationService<'_> {
    fn notify(&self, event: &NotificationEvent) {
        let result = self
            .app
            .notification()
            .builder()
            .title(&self.device_name)
            .body(&event.message)
            .show();

        if let Err(err) = result {
            log::warn!(
                "failed to show a notification for '{}': {err}",
                self.device_name
            );
        }
    }
}
