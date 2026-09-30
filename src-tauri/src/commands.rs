//! 画面から呼ぶコマンド。

use crate::config::{same_dir, Profile, Settings};
use crate::credentials;
use crate::discovery;
use crate::history::Point;
use crate::state::{self, AppState, ConsentStage, RefreshKind, StateView};
use crate::windows::{self, Popover};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

fn changed(app: &AppHandle) {
    crate::tray::sync(app);
    let _ = app.emit("state-changed", ());
}

#[tauri::command]
pub fn get_state(state: State<'_, AppState>) -> StateView {
    state::view(&state)
}

#[tauri::command]
pub fn refresh_now(state: State<'_, AppState>) {
    state.request_refresh(RefreshKind::Force);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PopoverInfo {
    profile_id: Option<String>,
    pinned: bool,
}

#[tauri::command]
pub fn popover_info(popover: State<'_, Popover>) -> PopoverInfo {
    let p = popover.lock();
    PopoverInfo {
        profile_id: p.profile_id.clone(),
        pinned: p.pinned,
    }
}

#[tauri::command]
pub fn select_popover_profile(popover: State<'_, Popover>, id: String) {
    popover.lock().profile_id = Some(id);
}

#[tauri::command]
pub fn fit_popover(app: AppHandle, height: f64) {
    windows::fit_popover(&app, height.clamp(120.0, 2000.0));
}

#[tauri::command]
pub fn set_pinned(app: AppHandle, pinned: bool) {
    windows::set_pinned(&app, pinned);
}

#[tauri::command]
pub fn hide_popover(app: AppHandle) {
    windows::hide_popover(&app);
}

#[tauri::command]
pub fn open_settings(app: AppHandle) {
    windows::open_settings(&app);
}

/// 開けるのは決まったページだけにする。
#[tauri::command]
pub fn open_link(app: AppHandle, key: String) -> Result<(), String> {
    let url = match key.as_str() {
        "status" => "https://status.claude.com",
        "repo" => "https://github.com/nanase/mira",
        "original" => "https://github.com/hamed-elfayome/Claude-Usage-Tracker",
        _ => return Err(format!("unknown link: {key}")),
    };
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<(), String> {
    let settings = settings.normalized();
    let autostart_changed = {
        let inner = state.lock();
        inner.config.settings.autostart != settings.autostart
    };
    if autostart_changed {
        let autolaunch = app.autolaunch();
        let result = if settings.autostart {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        result.map_err(|e| e.to_string())?;
    }
    {
        let mut inner = state.lock();
        inner.config.settings = settings;
        state.save(&inner);
    }
    windows::apply_theme(&app);
    // 更新間隔が変わったかもしれないので、ループに見直させる
    state.notify.notify_one();
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn rename_profile(app: AppHandle, state: State<'_, AppState>, id: String, name: String) {
    let name = name.trim().to_owned();
    if name.is_empty() {
        return;
    }
    {
        let mut inner = state.lock();
        if let Some(p) = inner.config.profile_mut(&id) {
            p.name = name;
        }
        state.save(&inner);
    }
    changed(&app);
}

/// プロファイルを外す。ディレクトリの中身には触れない。自動検出でも再び尋ねない。
#[tauri::command]
pub fn remove_profile(app: AppHandle, state: State<'_, AppState>, id: String) {
    {
        let mut inner = state.lock();
        if let Some(pos) = inner.config.profiles.iter().position(|p| p.id == id) {
            let removed = inner.config.profiles.remove(pos);
            if !inner
                .config
                .declined_dirs
                .iter()
                .any(|d| same_dir(d, &removed.dir))
            {
                inner.config.declined_dirs.push(removed.dir);
            }
        }
        inner.runtime.remove(&id);
        inner
            .consent
            .retain(|c| !matches!(c, ConsentStage::Refresh { profile_id } if *profile_id == id));
        state.save(&inner);
    }
    state.history.remove(&id);
    {
        let popover = app.state::<Popover>();
        let mut p = popover.lock();
        if p.profile_id.as_deref() == Some(id.as_str()) {
            p.profile_id = None;
        }
    }
    changed(&app);
}

#[tauri::command]
pub fn set_token_refresh(app: AppHandle, state: State<'_, AppState>, id: String, allow: bool) {
    {
        let mut inner = state.lock();
        if let Some(p) = inner.config.profile_mut(&id) {
            p.allow_token_refresh = Some(allow);
        }
        state.save(&inner);
    }
    changed(&app);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AddResult {
    Queued,
    AlreadyRegistered,
    NoCredentials,
    Cancelled,
}

/// フォルダを選んでもらい、登録の確認ダイアログに回す。
#[tauri::command]
pub async fn add_profile_folder(app: AppHandle) -> AddResult {
    let state = app.state::<AppState>();
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title(crate::i18n::texts(state.lang).pick_folder)
        .set_directory(&state.home)
        .pick_folder(move |folder| {
            let _ = tx.send(folder.and_then(|f| f.into_path().ok()));
        });
    let Ok(Some(dir)) = rx.await else {
        return AddResult::Cancelled;
    };
    if !credentials::exists(&dir) {
        return AddResult::NoCredentials;
    }
    {
        let mut inner = state.lock();
        if inner.config.profiles.iter().any(|p| same_dir(&p.dir, &dir)) {
            return AddResult::AlreadyRegistered;
        }
        inner.config.declined_dirs.retain(|d| !same_dir(d, &dir));
        inner.consent.push_front(ConsentStage::Register { dir });
    }
    windows::open_consent(&app);
    AddResult::Queued
}

/// 自動検出をやり直す。見つかれば確認ダイアログを出す。
#[tauri::command]
pub fn rescan_profiles(app: AppHandle, state: State<'_, AppState>) -> bool {
    let found = state.queue_consents();
    if found {
        windows::open_consent(&app);
    }
    found
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentView {
    step: u8,
    dir_display: String,
    credentials_display: String,
    default_name: String,
    profile_name: String,
}

#[tauri::command]
pub fn get_consent(state: State<'_, AppState>) -> Option<ConsentView> {
    let mut inner = state.lock();
    loop {
        let view = match inner.consent.front()? {
            ConsentStage::Register { dir } => Some(ConsentView {
                step: 1,
                dir_display: discovery::display_path(dir, &state.home),
                credentials_display: discovery::display_path(
                    &credentials::credentials_path(dir),
                    &state.home,
                ),
                default_name: discovery::default_name(dir),
                profile_name: String::new(),
            }),
            ConsentStage::Refresh { profile_id } => {
                inner.config.profile(profile_id).map(|p| ConsentView {
                    step: 2,
                    dir_display: discovery::display_path(&p.dir, &state.home),
                    credentials_display: discovery::display_path(
                        &credentials::credentials_path(&p.dir),
                        &state.home,
                    ),
                    default_name: String::new(),
                    profile_name: p.name.clone(),
                })
            }
        };
        match view {
            Some(view) => return Some(view),
            // 途中で外されたプロファイルの分は飛ばす
            None => {
                inner.consent.pop_front();
            }
        }
    }
}

fn after_consent(app: &AppHandle, state: &AppState) {
    let empty = state.lock().consent.is_empty();
    if empty {
        windows::close_consent(app);
    } else {
        let _ = app.emit_to(windows::CONSENT, "consent-changed", ());
    }
    changed(app);
}

#[tauri::command]
pub fn consent_register(app: AppHandle, state: State<'_, AppState>, name: String) {
    {
        let mut inner = state.lock();
        let Some(ConsentStage::Register { dir }) = inner.consent.front().cloned() else {
            return;
        };
        inner.consent.pop_front();
        let name = match name.trim() {
            "" => discovery::default_name(&dir),
            n => n.to_owned(),
        };
        let id = uuid::Uuid::new_v4().to_string();
        inner.config.profiles.push(Profile {
            id: id.clone(),
            name,
            dir,
            allow_token_refresh: None,
        });
        inner
            .consent
            .push_front(ConsentStage::Refresh { profile_id: id });
        state.save(&inner);
    }
    state.request_refresh(RefreshKind::Soft);
    after_consent(&app, &state);
}

#[tauri::command]
pub fn consent_decline(app: AppHandle, state: State<'_, AppState>) {
    {
        let mut inner = state.lock();
        let Some(ConsentStage::Register { dir }) = inner.consent.front().cloned() else {
            return;
        };
        inner.consent.pop_front();
        inner.config.declined_dirs.push(dir);
        state.save(&inner);
    }
    after_consent(&app, &state);
}

#[tauri::command]
pub fn consent_refresh(app: AppHandle, state: State<'_, AppState>, allow: bool) {
    {
        let mut inner = state.lock();
        let Some(ConsentStage::Refresh { profile_id }) = inner.consent.front().cloned() else {
            return;
        };
        inner.consent.pop_front();
        if let Some(p) = inner.config.profile_mut(&profile_id) {
            p.allow_token_refresh = Some(allow);
        }
        state.save(&inner);
    }
    state.request_refresh(RefreshKind::Soft);
    after_consent(&app, &state);
}

#[tauri::command]
pub fn fit_consent(app: AppHandle, height: f64) {
    windows::fit_consent(&app, height.clamp(200.0, 900.0));
}

#[tauri::command]
pub fn get_history(state: State<'_, AppState>, id: String, days: u32) -> Vec<Point> {
    let since = chrono::Utc::now().timestamp_millis() - days.clamp(1, 90) as i64 * 86_400_000;
    state.history.read(&id, since)
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}
