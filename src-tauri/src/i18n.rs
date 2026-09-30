//! Rust 側で出す文言（トレイのツールチップとメニュー）。画面の文言は ui/i18n.js にある。

use chrono::{DateTime, Local, Utc};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Lang {
    Ja,
    En,
}

impl Lang {
    pub fn detect() -> Lang {
        match sys_locale::get_locale() {
            Some(locale) if locale.to_lowercase().starts_with("ja") => Lang::Ja,
            _ => Lang::En,
        }
    }
}

pub struct Texts {
    pub open: &'static str,
    pub refresh: &'static str,
    pub settings: &'static str,
    pub quit: &'static str,
    pub session: &'static str,
    pub weekly: &'static str,
    pub loading: &'static str,
    pub auth_expired: &'static str,
    pub fetch_failed: &'static str,
    pub no_profiles: &'static str,
    pub pick_folder: &'static str,
}

pub fn texts(lang: Lang) -> &'static Texts {
    match lang {
        Lang::Ja => &Texts {
            open: "開く",
            refresh: "今すぐ更新",
            settings: "設定",
            quit: "Mira を終了",
            session: "セッション",
            weekly: "週間",
            loading: "取得中…",
            auth_expired: "認証が切れています",
            fetch_failed: "取得できませんでした",
            no_profiles: "プロファイルがありません。クリックして設定を開く",
            pick_folder: "Claude Code の設定フォルダを選ぶ",
        },
        Lang::En => &Texts {
            open: "Open",
            refresh: "Refresh now",
            settings: "Settings",
            quit: "Quit Mira",
            session: "Session",
            weekly: "Weekly",
            loading: "Loading…",
            auth_expired: "Sign-in expired",
            fetch_failed: "Couldn't fetch usage",
            no_profiles: "No profiles. Click to open settings",
            pick_folder: "Choose a Claude Code config folder",
        },
    }
}

/// 今日なら「16:40」、それ以外は「10/2 02:00」。
pub fn short_time(t: DateTime<Utc>) -> String {
    let local = t.with_timezone(&Local);
    if local.date_naive() == Local::now().date_naive() {
        local.format("%H:%M").to_string()
    } else {
        local.format("%-m/%-d %H:%M").to_string()
    }
}

/// ツールチップの 1 行。例: 「セッション 74%（17:20 にリセット）」
pub fn meter_line(
    lang: Lang,
    label: &str,
    used: f64,
    show_remaining: bool,
    resets_at: Option<DateTime<Utc>>,
) -> String {
    let value = if show_remaining {
        (100.0 - used).max(0.0)
    } else {
        used
    }
    .round() as i64;
    let reset = resets_at.map(short_time);
    match (lang, show_remaining, reset) {
        (Lang::Ja, false, Some(r)) => format!("{label} {value}%（{r} にリセット）"),
        (Lang::Ja, false, None) => format!("{label} {value}%"),
        (Lang::Ja, true, Some(r)) => format!("{label} 残り {value}%（{r} にリセット）"),
        (Lang::Ja, true, None) => format!("{label} 残り {value}%"),
        (Lang::En, false, Some(r)) => format!("{label} {value}% (resets {r})"),
        (Lang::En, false, None) => format!("{label} {value}%"),
        (Lang::En, true, Some(r)) => format!("{label} {value}% left (resets {r})"),
        (Lang::En, true, None) => format!("{label} {value}% left"),
    }
}

/// Windows のツールチップは 127 文字までなので切り詰める。
pub fn clamp_tooltip(s: String) -> String {
    if s.chars().count() <= 127 {
        s
    } else {
        s.chars().take(126).collect::<String>() + "…"
    }
}
