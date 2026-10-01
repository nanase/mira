//! トレイアイコン。プロファイルごとに 1 つ出す。プロファイルがなければ 1 つだけ出して設定へ誘導する。

use crate::config::IconStyle;
use crate::i18n::{self, Lang};
use crate::icon::{self, IconSpec, Meter};
use crate::state::{self, AppState, RefreshKind};
use crate::windows;
use chrono::{DateTime, Utc};
use std::sync::Mutex;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

const EMPTY_ID: &str = "mira-empty";

/// 作ったトレイの ID。消すときに使う。
#[derive(Default)]
pub struct TrayIds(Mutex<Vec<String>>);

pub fn tray_id(profile_id: &str) -> String {
    format!("mira-{profile_id}")
}

struct TrayContent {
    id: String,
    profile_id: Option<String>,
    rgba: Vec<u8>,
    size: u32,
    tooltip: String,
}

/// トレイの数と見た目を現在の状態に合わせる。
pub fn sync(app: &AppHandle) {
    let contents = build_contents(app);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || apply(&handle, contents));
}

fn build_contents(app: &AppHandle) -> Vec<TrayContent> {
    let state = app.state::<AppState>();
    let lang = state.lang;
    let t = i18n::texts(lang);
    let size = crate::platform::tray_icon_size();
    let dark = crate::platform::taskbar_is_dark();
    let inner = state.lock();
    let settings = &inner.config.settings;
    let mono = settings.color_mode == crate::config::ColorMode::Mono;
    let now = Utc::now();

    if inner.config.profiles.is_empty() {
        let spec = IconSpec {
            style: IconStyle::Ring,
            size,
            dark_taskbar: dark,
            mono,
            session: None,
            weekly: None,
            show_marker: false,
            pace_marker: false,
            letter: None,
            number: None,
        };
        return vec![TrayContent {
            id: EMPTY_ID.into(),
            profile_id: None,
            rgba: icon::render(&spec, &state.fonts),
            size,
            tooltip: format!("Mira\n{}", t.no_profiles),
        }];
    }

    let multi = inner.config.profiles.len() > 1;
    inner
        .config
        .profiles
        .iter()
        .map(|p| {
            let rt = inner.runtime.get(&p.id);
            let usage = rt.and_then(|r| r.usage.as_ref());
            let rows = usage
                .map(|u| state::rows(u, now, settings.show_remaining))
                .unwrap_or_default();
            let find = |kind: &str| rows.iter().find(|r| r.kind == kind);
            let meter = |r: &state::RowView| Meter {
                percent: if settings.show_remaining {
                    (100.0 - r.used).max(0.0)
                } else {
                    r.used
                },
                level: r.level,
                marker: r
                    .elapsed
                    .map(|e| if settings.show_remaining { 1.0 - e } else { e }),
                pace: r.pace,
            };
            let session = find("session").map(meter);
            let spec = IconSpec {
                style: settings.icon_style,
                size,
                dark_taskbar: dark,
                mono,
                session,
                weekly: find("weekly").map(meter),
                show_marker: settings.tray_marker,
                pace_marker: settings.marker_mode == crate::config::MarkerMode::Pace,
                letter: if multi {
                    p.name
                        .chars()
                        .next()
                        .map(|c| c.to_uppercase().next().unwrap_or(c))
                } else {
                    None
                },
                number: session.map(|m| m.percent.round() as u32),
            };

            let mut tooltip = format!("Mira · {}", p.name);
            if rt.is_some_and(|r| r.auth_error) {
                tooltip += &format!("\n{}", t.auth_expired);
            } else if usage.is_none() {
                let failed = rt.is_some_and(|r| r.failures > 0);
                tooltip += &format!("\n{}", if failed { t.fetch_failed } else { t.loading });
            } else {
                for (kind, label) in [("session", t.session), ("weekly", t.weekly)] {
                    if let Some(r) = find(kind) {
                        let resets = r
                            .resets_at_ms
                            .and_then(DateTime::<Utc>::from_timestamp_millis);
                        tooltip += &format!(
                            "\n{}",
                            i18n::meter_line(lang, label, r.used, settings.show_remaining, resets)
                        );
                    }
                }
            }
            TrayContent {
                id: tray_id(&p.id),
                profile_id: Some(p.id.clone()),
                rgba: icon::render(&spec, &state.fonts),
                size,
                tooltip: i18n::clamp_tooltip(tooltip),
            }
        })
        .collect()
}

fn apply(app: &AppHandle, contents: Vec<TrayContent>) {
    let ids = app.state::<TrayIds>();
    let mut ids = ids.0.lock().unwrap_or_else(|e| e.into_inner());
    ids.retain(|id| {
        let keep = contents.iter().any(|c| &c.id == id);
        if !keep {
            let _ = app.remove_tray_by_id(id.as_str());
        }
        keep
    });
    let lang = app.state::<AppState>().lang;
    for content in contents {
        let image = Image::new(&content.rgba, content.size, content.size);
        if let Some(tray) = app.tray_by_id(content.id.as_str()) {
            let _ = tray.set_icon(Some(image));
            let _ = tray.set_tooltip(Some(&content.tooltip));
            continue;
        }
        match create(app, lang, &content, image) {
            Ok(()) => ids.push(content.id.clone()),
            Err(err) => eprintln!("トレイアイコンを作れませんでした: {err}"),
        }
    }
}

fn create(
    app: &AppHandle,
    lang: Lang,
    content: &TrayContent,
    image: Image<'_>,
) -> tauri::Result<()> {
    let t = i18n::texts(lang);
    // メニューのイベントはアプリ全体に届くので、ID にトレイの ID を含めて自分の分だけ処理する
    let tid = content.id.clone();
    let item = |action: &str, label: &str| {
        MenuItem::with_id(app, format!("{tid}|{action}"), label, true, None::<&str>)
    };
    let settings = item("settings", t.settings)?;
    let quit = item("quit", t.quit)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = if content.profile_id.is_some() {
        let open = item("open", t.open)?;
        let refresh = item("refresh", t.refresh)?;
        Menu::with_items(app, &[&open, &refresh, &settings, &separator, &quit])?
    } else {
        Menu::with_items(app, &[&settings, &separator, &quit])?
    };

    let profile_id = content.profile_id.clone();
    let menu_profile_id = content.profile_id.clone();
    let menu_tid = tid.clone();
    TrayIconBuilder::with_id(tid)
        .icon(image)
        .tooltip(&content.tooltip)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            let Some((owner, action)) = event.id().as_ref().split_once('|') else {
                return;
            };
            if owner != menu_tid {
                return;
            }
            match action {
                "open" => {
                    if let Some(pid) = &menu_profile_id {
                        windows::toggle_popover(app, pid, None);
                    }
                }
                "refresh" => app.state::<AppState>().request_refresh(RefreshKind::Force),
                "settings" => windows::open_settings(app),
                "quit" => app.exit(0),
                _ => {}
            }
        })
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                match &profile_id {
                    Some(pid) => windows::toggle_popover(tray.app_handle(), pid, Some(rect)),
                    None => windows::open_settings(tray.app_handle()),
                }
            }
        })
        .build(app)?;
    Ok(())
}
