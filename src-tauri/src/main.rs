#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod git;
mod github;
mod license;
mod theme;

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

/// PR + CI status for the current branch (issue #3). Returns None when there's
/// no token, no GitHub origin, or no open PR - the panel then shows no badge.
#[tauri::command]
async fn pr_status(path: String) -> Result<Option<github::PrStatus>, String> {
    github::pr_status(&path).await
}

/// Open an external URL in the user's browser (e.g. the PR page).
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("refusing to open a non-http URL".into());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(&url).spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &url])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(&url).spawn().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// User-authored themes from the config dir's `themes/` folder (issue #6).
/// Cross-platform: `app_config_dir` resolves to Application Support / AppData /
/// ~/.config per OS.
#[tauri::command]
fn load_themes(app: tauri::AppHandle) -> Result<Vec<theme::DiskTheme>, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("themes");
    Ok(theme::parse_dir(&dir))
}

#[cfg_attr(feature = "appstore", allow(dead_code))]
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// Secure storage (OS Keychain / Credential Manager / secret-service). Used to
// pin the trial start and the license so deleting a plain file can't reset the
// trial. Falls back silently when no secret service is available.
#[cfg(not(feature = "appstore"))]
const KEYRING_SERVICE: &str = "dev.peterdsp.glint";

#[cfg(not(feature = "appstore"))]
fn kv_get(key: &str) -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, key)
        .ok()?
        .get_password()
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(not(feature = "appstore"))]
fn kv_set(key: &str, val: &str) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, key) {
        let _ = entry.set_password(val);
    }
}

/// Trial / license state. The App Store build is always "licensed" (Apple gates
/// the purchase); the direct build tracks a 7-day trial then requires a key.
#[tauri::command]
fn license_status(app: tauri::AppHandle) -> Result<license::LicenseState, String> {
    #[cfg(feature = "appstore")]
    {
        let _ = app;
        return Ok(license::licensed_state());
    }
    #[cfg(not(feature = "appstore"))]
    {
        let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir).ok();
        let now = now_secs();

        // Trial start = the EARLIEST timestamp found in the Keychain or the
        // config file. Clearing one store won't reset the trial; both must go,
        // and the Keychain entry is hard to find. On first run, seed both.
        let fr_path = dir.join("first_run");
        let file_fr = std::fs::read_to_string(&fr_path)
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok());
        let key_fr = kv_get("first_run").and_then(|s| s.parse::<u64>().ok());
        let first_run = [file_fr, key_fr].into_iter().flatten().min().unwrap_or(now);
        let _ = std::fs::write(&fr_path, first_run.to_string());
        kv_set("first_run", &first_run.to_string());

        // License: Keychain first, then the config file.
        let license = kv_get("license")
            .or_else(|| std::fs::read_to_string(dir.join("license.key")).ok())
            .and_then(|k| license::verify(k.trim(), license::pubkey_b64()).ok());

        Ok(license::evaluate(first_run, now, license))
    }
}

/// Validate and store a license key, returning the new state.
#[tauri::command]
fn activate_license(app: tauri::AppHandle, key: String) -> Result<license::LicenseState, String> {
    #[cfg(feature = "appstore")]
    {
        let _ = (app, key);
        return Ok(license::licensed_state());
    }
    #[cfg(not(feature = "appstore"))]
    {
        license::verify(key.trim(), license::pubkey_b64())?;
        let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir).ok();
        std::fs::write(dir.join("license.key"), key.trim()).ok();
        kv_set("license", key.trim());
        license_status(app)
    }
}

#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Silent background update (direct build): check GitHub Releases, and if a
/// newer signed build exists, download + install it and relaunch - no prompts,
/// no admin password (the app is signed/notarized; see docs/RELEASE.md).
/// Returns true if an update was applied. The App Store build is a no-op.
#[cfg(feature = "updater")]
#[tauri::command]
async fn update_now(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater().map_err(|e| e.to_string())?;
    let Some(update) = updater.check().await.map_err(|e| e.to_string())? else {
        return Ok(false);
    };
    update
        .download_and_install(|_downloaded, _total| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    app.restart();
}

#[cfg(not(feature = "updater"))]
#[tauri::command]
async fn update_now() -> Result<bool, String> {
    Ok(false)
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
        .title(format!("Glint - {file}"))
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
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_positioner::init());

    // Self-updater only in the direct (Ko-fi) build; the App Store build omits
    // the feature entirely.
    #[cfg(feature = "updater")]
    {
        builder = builder
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_process::init());
    }

    builder
        .invoke_handler(tauri::generate_handler![
            get_status, commit, fetch, pull, push, diff, open_diff, load_themes,
            pr_status, open_url, license_status, activate_license, update_now,
            app_version
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

            // Windows: Mica gives an equivalent frosted backdrop. (Linux has no
            // native blur - the panel's CSS tint carries it there.)
            #[cfg(target_os = "windows")]
            {
                let _ = window_vibrancy::apply_mica(&win, None);
            }

            let quit = MenuItem::with_id(app, "quit", "Quit Glint", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            // Menu-bar icon: a monochrome template on macOS (the system tints it
            // for light/dark); the colored app icon on Windows/Linux, where
            // template icons aren't a concept.
            let mut tray = TrayIconBuilder::with_id("glint-tray")
                .menu(&menu)
                .show_menu_on_left_click(false);

            #[cfg(target_os = "macos")]
            {
                let icon = tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/tray-template.png"
                ))?;
                tray = tray.icon(icon).icon_as_template(true);
            }
            #[cfg(not(target_os = "macos"))]
            {
                tray = tray.icon(app.default_window_icon().unwrap().clone());
            }

            let _tray = tray
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
