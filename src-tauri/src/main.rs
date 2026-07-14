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

/// Native folder picker for connecting a repository. Returns the chosen path,
/// or None if the user cancels. The frontend then validates it is a real Git
/// repo by loading its status.
#[tauri::command]
fn pick_repo(app: tauri::AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog()
        .file()
        .set_title("Choose a Git repository")
        .blocking_pick_folder()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

// ---------------------------------------------------------------------------
// Presentation mode: a menu-bar dropdown, or a regular Dock window. Chosen on
// first launch and stored in a small config file so the Rust side can read it
// at startup - before the webview loads - to set the macOS activation policy
// and window style.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    MenuBar,
    Dock,
    FirstRun,
}

fn mode_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("mode"))
}

fn read_mode(app: &tauri::AppHandle) -> Mode {
    let raw = mode_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.trim().to_string());
    match raw.as_deref() {
        Some("dock") => Mode::Dock,
        Some("menubar") => Mode::MenuBar,
        _ => Mode::FirstRun,
    }
}

/// The chosen presentation mode ("menubar" | "dock"), or null on first run so
/// the UI can show the chooser.
#[tauri::command]
fn app_mode(app: tauri::AppHandle) -> Option<String> {
    match read_mode(&app) {
        Mode::MenuBar => Some("menubar".into()),
        Mode::Dock => Some("dock".into()),
        Mode::FirstRun => None,
    }
}

/// Persist the presentation mode and relaunch so it applies cleanly from
/// startup (activation policy and window style are set before the UI loads).
#[tauri::command]
fn set_app_mode(app: tauri::AppHandle, mode: String) -> Result<(), String> {
    let m = if mode == "dock" { "dock" } else { "menubar" };
    if let Some(p) = mode_path(&app) {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        std::fs::write(&p, m).map_err(|e| e.to_string())?;
    }
    app.restart()
}

/// Store (empty string clears) a GitHub token in the OS Keychain. The sandboxed
/// App Store build authenticates HTTPS git and PR/CI status entirely from this,
/// since it cannot reach `~/.ssh`, the credential helper, or the gh CLI. Other
/// builds treat it as an optional fallback ahead of the gh CLI.
#[tauri::command]
fn set_github_token(token: String) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, "gh_token").map_err(|e| e.to_string())?;
    let t = token.trim();
    if t.is_empty() {
        let _ = entry.delete_credential();
        Ok(())
    } else {
        entry.set_password(t).map_err(|e| e.to_string())
    }
}

/// Whether a GitHub token is currently stored. Never returns the token itself,
/// so Settings can show its state without the secret round-tripping to the UI.
#[tauri::command]
fn github_token_set() -> bool {
    crate::github::stored_github_token().is_some()
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
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_dialog::init());

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
            app_version, set_github_token, github_token_set, pick_repo,
            app_mode, set_app_mode
        ])
        .setup(|app| {
            let win = app
                .get_webview_window("panel")
                .expect("panel window missing");

            let mode = read_mode(app.handle());

            // macOS activation policy: a menu-bar app has no Dock icon
            // (Accessory); a Dock app - and the first-run chooser, so it shows
            // as a normal foreground window - is Regular.
            #[cfg(target_os = "macos")]
            {
                use tauri::ActivationPolicy;
                let policy = match mode {
                    Mode::MenuBar => ActivationPolicy::Accessory,
                    _ => ActivationPolicy::Regular,
                };
                let _ = app.set_activation_policy(policy);
            }

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

            // Window presentation per mode. The menu-bar panel stays hidden
            // until the tray is clicked; the Dock window (and the first-run
            // chooser) are shown up front.
            match mode {
                Mode::Dock => {
                    let _ = win.set_decorations(true);
                    let _ = win.set_always_on_top(false);
                    let _ = win.set_resizable(true);
                    let _ = win.set_min_size(Some(tauri::LogicalSize::new(340.0, 480.0)));
                    let _ = win.set_size(tauri::LogicalSize::new(380.0, 620.0));
                    let _ = win.center();
                    let _ = win.show();
                    let _ = win.set_focus();
                }
                Mode::FirstRun => {
                    let _ = win.center();
                    let _ = win.show();
                    let _ = win.set_focus();
                }
                Mode::MenuBar => {}
            }

            // The tray icon only exists in menu-bar mode; a Dock app doesn't
            // need one, and the first-run window is already visible.
            if mode == Mode::MenuBar {
                let quit = MenuItem::with_id(app, "quit", "Quit Glint", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&quit])?;

                // Menu-bar icon: a monochrome template on macOS (the system
                // tints it for light/dark); the colored app icon elsewhere,
                // where template icons aren't a concept.
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
            }

            Ok(())
        })
        .on_window_event(|win, event| match event {
            // Menu-bar UX: dismiss the panel when it loses focus. In Dock mode
            // it is a normal window, so leave it be.
            tauri::WindowEvent::Focused(false)
                if read_mode(win.app_handle()) == Mode::MenuBar =>
            {
                let _ = win.hide();
            }
            // Dock mode: closing the window hides it (the app stays in the
            // Dock) rather than quitting.
            tauri::WindowEvent::CloseRequested { api, .. }
                if read_mode(win.app_handle()) == Mode::Dock =>
            {
                api.prevent_close();
                let _ = win.hide();
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building Glint")
        .run(|app_handle, event| {
            // Dock mode: clicking the Dock icon with the window closed
            // (hidden) reopens it, the way a normal Mac app behaves.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                if let Some(win) = app_handle.get_webview_window("panel") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (app_handle, event);
        });
}
