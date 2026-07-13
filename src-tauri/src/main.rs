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

#[tauri::command]
fn fetch(path: String) -> Result<git::RepoStatus, String> {
    git::fetch(&path)
}

#[tauri::command]
fn pull(path: String) -> Result<git::RepoStatus, String> {
    git::pull(&path)
}

#[tauri::command]
fn push(path: String) -> Result<git::RepoStatus, String> {
    git::push(&path)
}

#[tauri::command]
fn diff(path: String, file: String) -> Result<git::FileDiff, String> {
    git::diff(&path, &file)
}

/// Open (or refocus) the diff pop-out for a file. The panel is a transient
/// menu-bar dropdown; diffs want a real resizable window, so this is separate.
#[tauri::command]
fn open_diff(app: tauri::AppHandle, path: String, file: String) -> Result<(), String> {
    let repo_js = serde_json::to_string(&path).map_err(|e| e.to_string())?;
    let file_js = serde_json::to_string(&file).map_err(|e| e.to_string())?;

    // Reuse an already-open diff window rather than stacking new ones.
    if let Some(win) = app.get_webview_window("diff") {
        let _ = win.eval(&format!(
            "window.__glintShowDiff && window.__glintShowDiff({repo_js}, {file_js})"
        ));
        let _ = win.set_focus();
        return Ok(());
    }

    // Seed the target before the page scripts run, then load the diff view.
    let init = format!("window.__GLINT_DIFF__ = {{ repo: {repo_js}, file: {file_js} }};");
    tauri::WebviewWindowBuilder::new(&app, "diff", tauri::WebviewUrl::App("diff.html".into()))
        .title(format!("Glint — {file}"))
        .inner_size(780.0, 620.0)
        .min_inner_size(460.0, 320.0)
        .initialization_script(&init)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
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
        .invoke_handler(tauri::generate_handler![
            get_status, commit, fetch, pull, push, diff, open_diff
        ])
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

            // Monochrome template icon: macOS tints it for light/dark menu bars,
            // unlike the colored app icon. Embedded so there's no runtime file.
            let tray_icon = tauri::image::Image::from_bytes(include_bytes!(
                "../icons/tray-template.png"
            ))?;

            let _tray = TrayIconBuilder::with_id("glint-tray")
                .icon(tray_icon)
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
