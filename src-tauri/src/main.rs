#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod git;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewWindow,
};
use tauri_plugin_positioner::{Position, WindowExt};

#[tauri::command]
fn get_status(path: String) -> Result<git::RepoStatus, String> {
    git::get_status(&path)
}

#[tauri::command]
fn commit(
    path: String,
    files: Vec<String>,
    summary: String,
    description: String,
) -> Result<(), String> {
    git::commit(&path, &files, &summary, &description)
}

fn toggle_panel(win: &WebviewWindow) {
    if win.is_visible().unwrap_or(false) {
        let _ = win.hide();
    } else {
        let _ = win.move_window(Position::TrayBottomCenter);
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .invoke_handler(tauri::generate_handler![get_status, commit])
        .setup(|app| {
            let win = app
                .get_webview_window("panel")
                .expect("panel window missing");

            // Native macOS translucency (NSVisualEffectView) behind the webview.
            // The web layer paints its themed glass tint on top of this.
            #[cfg(target_os = "macos")]
            {
                use window_vibrancy::{
                    apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState,
                };
                let _ = apply_vibrancy(
                    &win,
                    NSVisualEffectMaterial::HudWindow,
                    Some(NSVisualEffectState::Active),
                    Some(16.0),
                );
            }

            let quit = MenuItem::with_id(app, "quit", "Quit Glint", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            let _tray = TrayIconBuilder::with_id("glint-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(win) = tray.app_handle().get_webview_window("panel") {
                            toggle_panel(&win);
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        // Menu-bar UX: dismiss the panel when it loses focus.
        .on_window_event(|win, event| {
            if let tauri::WindowEvent::Focused(false) = event {
                let _ = win.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Glint");
}
