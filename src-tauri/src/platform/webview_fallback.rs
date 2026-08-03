#![cfg(windows)]

use tauri::{AppHandle, Manager};
use webview2_com::NavigationCompletedEventHandler;
use windows::core::{BOOL, HSTRING};

const FALLBACK_HTML: &str = include_str!("../../fallback.html");

/// Registers a WebView2 `NavigationCompleted` handler on the main window
/// that swaps in a small Pi-Hub-branded page whenever the window's content
/// fails to load, instead of leaving WebView2's own native network-error
/// chrome visible inside the app. The app only ever navigates once (its
/// initial load); it never links out to other sites from inside the
/// webview (services open externally via the opener plugin instead), so
/// any failed navigation here unambiguously means "the interface couldn't
/// load," not "the user navigated somewhere that 404'd."
pub fn install(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let result = window.with_webview(|webview| {
        let controller = webview.controller();
        let core = match unsafe { controller.CoreWebView2() } {
            Ok(core) => core,
            Err(err) => {
                log::warn!(
                    "could not access the WebView2 core to install the fallback-page handler: {err}"
                );
                return;
            }
        };

        let handler = NavigationCompletedEventHandler::create(Box::new(|sender, args| {
            let Some(args) = args else { return Ok(()) };
            let mut success = BOOL(0);
            unsafe { args.IsSuccess(&mut success)? };
            if !success.as_bool() {
                if let Some(webview) = sender {
                    unsafe { webview.NavigateToString(&HSTRING::from(FALLBACK_HTML))? };
                }
            }
            Ok(())
        }));

        let mut token: i64 = 0;
        if let Err(err) = unsafe { core.add_NavigationCompleted(&handler, &mut token) } {
            log::warn!("could not register the WebView2 fallback-page handler: {err}");
        }
    });

    if let Err(err) = result {
        log::warn!(
            "could not access the platform webview to install the fallback-page handler: {err}"
        );
    }
}
