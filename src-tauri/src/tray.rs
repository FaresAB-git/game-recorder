use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::state::AppState;

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Afficher", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quitter GameRecorder", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let mut builder = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("GameRecorder")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "quit" => quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Quit, but save the game first if one is being recorded (otherwise FFmpeg
/// would be killed and the MKV left half-written).
fn quit(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        if state.engine.is_recording().await {
            log::info!("Quit requested during a recording — saving it first");
            crate::detection::handle_game_end(&app).await;
        }
        // A game may already be finalizing in the detection loop: let it finish.
        for _ in 0..240 {
            if !state.recording_status.read().await.is_processing {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        app.exit(0);
    });
}
