//! アプリの状態、画面に渡すビュー、定期取得のループ。

use crate::api::{self, StatusIndicator, Usage};
use crate::config::{Config, Settings};
use crate::credentials::{self, CredError};
use crate::history::{History, Point};
use crate::i18n::Lang;
use crate::{discovery, icon, pace};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// プロファイルごとの取得状況。保存はしない。
#[derive(Default)]
pub struct Runtime {
    pub usage: Option<Usage>,
    pub last_success: Option<DateTime<Utc>>,
    pub last_attempt: Option<Instant>,
    pub failures: u32,
    pub auth_error: bool,
    /// 429 を受けたら、この時刻まで取得しない
    pub blocked_until: Option<Instant>,
}

/// 承諾ダイアログで尋ねる内容。
#[derive(Clone, Debug)]
pub enum ConsentStage {
    /// 手順 1: ディレクトリを登録するか
    Register { dir: PathBuf },
    /// 手順 2: トークンの書き戻しを許可するか
    Refresh { profile_id: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshKind {
    /// 定期取得。設定の間隔を空ける
    Scheduled,
    /// ポップアップを開いたとき。1 分以内に取得していれば省く
    Soft,
    /// 更新ボタン。429 を避けるため 10 秒は空ける
    Force,
}

pub struct Inner {
    pub config: Config,
    pub runtime: HashMap<String, Runtime>,
    pub status: Option<StatusIndicator>,
    pub status_fetched: Option<Instant>,
    pub request: Option<RefreshKind>,
    pub consent: VecDeque<ConsentStage>,
}

pub struct AppState {
    pub inner: Mutex<Inner>,
    pub config_path: PathBuf,
    pub home: PathBuf,
    pub history: History,
    pub client: reqwest::Client,
    pub notify: tokio::sync::Notify,
    pub token_lock: tokio::sync::Mutex<()>,
    pub fonts: icon::Fonts,
    pub lang: Lang,
}

impl AppState {
    pub fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn save(&self, inner: &Inner) {
        if let Err(err) = inner.config.save(&self.config_path) {
            eprintln!("設定を保存できませんでした: {err}");
        }
    }

    pub fn request_refresh(&self, kind: RefreshKind) {
        {
            let mut inner = self.lock();
            // 強い要求を弱い要求で上書きしない
            let stronger = match (inner.request, kind) {
                (Some(RefreshKind::Force), _) => RefreshKind::Force,
                (Some(RefreshKind::Soft), RefreshKind::Scheduled) => RefreshKind::Soft,
                _ => kind,
            };
            inner.request = Some(stronger);
        }
        self.notify.notify_one();
    }

    /// 自動検出で見つかった未登録のディレクトリと、承諾が未回答のプロファイルを積む。
    pub fn queue_consents(&self) -> bool {
        let mut inner = self.lock();
        for profile in inner.config.profiles.clone() {
            if profile.allow_token_refresh.is_none()
                && !inner.consent.iter().any(|c| matches!(c, ConsentStage::Refresh { profile_id } if *profile_id == profile.id))
            {
                inner.consent.push_back(ConsentStage::Refresh { profile_id: profile.id });
            }
        }
        for dir in discovery::candidates(&self.home) {
            let queued = inner
                .consent
                .iter()
                .any(|c| matches!(c, ConsentStage::Register { dir: d } if crate::config::same_dir(d, &dir)));
            if !queued && !inner.config.knows_dir(&dir) {
                inner.consent.push_back(ConsentStage::Register { dir });
            }
        }
        !inner.consent.is_empty()
    }
}

// ---- 画面に渡すビュー ----

/// リリース日。バージョンを上げるときに一緒に更新する（docs/building.md を参照）。
pub const RELEASE_DATE: &str = "2026-10-03";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateView {
    pub lang: Lang,
    pub version: &'static str,
    pub release_date: &'static str,
    pub settings: Settings,
    pub profiles: Vec<ProfileView>,
    pub status: Option<StatusIndicator>,
    pub now_ms: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileView {
    pub id: String,
    pub name: String,
    pub dir_display: String,
    pub allow_token_refresh: Option<bool>,
    pub rows: Vec<RowView>,
    pub balance: Option<MoneyView>,
    pub banner: Option<BannerView>,
    pub last_success_ms: Option<i64>,
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RowKind {
    Session,
    Weekly,
    Extra,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RowView {
    pub kind: RowKind,
    /// 使用率（0〜100）。残り表示への変換は画面側で行う
    pub used: f64,
    pub resets_at_ms: Option<i64>,
    /// 窓の経過率（0〜1）
    pub elapsed: Option<f64>,
    pub level: u8,
    pub pace: Option<u8>,
    pub extra_used: Option<MoneyView>,
    pub extra_limit: Option<MoneyView>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MoneyView {
    pub amount: f64,
    pub currency: String,
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum BannerKind {
    Expired,
    Failed,
    Stale,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BannerView {
    pub kind: BannerKind,
    pub failures: u32,
    pub minutes: i64,
}

fn money(m: &api::Money) -> MoneyView {
    MoneyView {
        amount: m.amount,
        currency: m.currency.clone(),
    }
}

fn window_row(
    kind: RowKind,
    w: &api::Window,
    duration: f64,
    now: DateTime<Utc>,
    show_remaining: bool,
) -> RowView {
    let used = w.percent_at(now);
    let elapsed = pace::elapsed_fraction(w.resets_at, duration, now);
    RowView {
        kind,
        used,
        resets_at_ms: w
            .resets_at
            .filter(|_| !w.expired(now))
            .map(|r| r.timestamp_millis()),
        elapsed,
        level: pace::level(used, elapsed, show_remaining),
        pace: pace::pace(used, elapsed),
        extra_used: None,
        extra_limit: None,
    }
}

pub fn rows(usage: &Usage, now: DateTime<Utc>, show_remaining: bool) -> Vec<RowView> {
    let mut rows = Vec::new();
    if let Some(w) = &usage.session {
        rows.push(window_row(
            RowKind::Session,
            w,
            pace::SESSION_SECS,
            now,
            show_remaining,
        ));
    }
    if let Some(w) = &usage.weekly {
        rows.push(window_row(
            RowKind::Weekly,
            w,
            pace::WEEK_SECS,
            now,
            show_remaining,
        ));
    }
    if let Some(extra) = &usage.extra {
        rows.push(RowView {
            kind: RowKind::Extra,
            used: extra.percent,
            resets_at_ms: None,
            elapsed: None,
            level: pace::level(extra.percent, None, show_remaining),
            pace: None,
            extra_used: Some(money(&extra.used)),
            extra_limit: Some(money(&extra.limit)),
        });
    }
    rows
}

pub fn banner(rt: &Runtime, settings: &Settings, now: DateTime<Utc>) -> Option<BannerView> {
    if rt.auth_error {
        return Some(BannerView {
            kind: BannerKind::Expired,
            failures: rt.failures,
            minutes: 0,
        });
    }
    if rt.failures >= 3 {
        return Some(BannerView {
            kind: BannerKind::Failed,
            failures: rt.failures,
            minutes: 0,
        });
    }
    let age = (now - rt.last_success?).num_minutes();
    (age > settings.refresh_minutes as i64 + 2).then_some(BannerView {
        kind: BannerKind::Stale,
        failures: rt.failures,
        minutes: age,
    })
}

pub fn view(state: &AppState) -> StateView {
    let inner = state.lock();
    let now = Utc::now();
    let settings = inner.config.settings.clone();
    let empty = Runtime::default();
    let profiles = inner
        .config
        .profiles
        .iter()
        .map(|p| {
            let rt = inner.runtime.get(&p.id).unwrap_or(&empty);
            ProfileView {
                id: p.id.clone(),
                name: p.name.clone(),
                dir_display: discovery::display_path(&p.dir, &state.home),
                allow_token_refresh: p.allow_token_refresh,
                rows: rt
                    .usage
                    .as_ref()
                    .map(|u| rows(u, now, settings.show_remaining))
                    .unwrap_or_default(),
                balance: rt
                    .usage
                    .as_ref()
                    .and_then(|u| u.balance.as_ref())
                    .map(money),
                banner: banner(rt, &settings, now),
                last_success_ms: rt.last_success.map(|t| t.timestamp_millis()),
            }
        })
        .collect();
    StateView {
        lang: state.lang,
        version: env!("CARGO_PKG_VERSION"),
        release_date: RELEASE_DATE,
        settings,
        profiles,
        status: inner.status,
        now_ms: now.timestamp_millis(),
    }
}

// ---- 定期取得 ----

pub async fn run(app: AppHandle) {
    let state = app.state::<AppState>();
    loop {
        refresh(&state).await;
        crate::tray::sync(&app);
        let _ = app.emit("state-changed", ());
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(60)) => {}
            _ = state.notify.notified() => {}
        }
    }
}

enum Failure {
    Cred(CredError),
    Fetch(api::FetchError),
}

async fn refresh(state: &AppState) {
    let (jobs, fetch_status, interval) = {
        let mut inner = state.lock();
        let kind = inner.request.take().unwrap_or(RefreshKind::Scheduled);
        let interval = Duration::from_secs(inner.config.settings.refresh_minutes as u64 * 60);
        let min_gap = match kind {
            RefreshKind::Scheduled => interval,
            RefreshKind::Soft => interval.min(Duration::from_secs(60)),
            RefreshKind::Force => Duration::from_secs(10),
        };
        let now = Instant::now();
        let mut jobs = Vec::new();
        for p in &inner.config.profiles {
            let rt = inner.runtime.get(&p.id);
            if rt.and_then(|r| r.blocked_until).is_some_and(|b| b > now) {
                continue;
            }
            if rt
                .and_then(|r| r.last_attempt)
                .is_none_or(|l| now - l >= min_gap)
            {
                jobs.push((
                    p.id.clone(),
                    p.dir.clone(),
                    p.allow_token_refresh == Some(true),
                ));
            }
        }
        for (id, _, _) in &jobs {
            inner.runtime.entry(id.clone()).or_default().last_attempt = Some(now);
        }
        let fetch_status = !jobs.is_empty()
            && inner.status_fetched.is_none_or(|t| {
                now - t >= interval.min(Duration::from_secs(300)) || kind == RefreshKind::Force
            });
        (jobs, fetch_status, interval)
    };

    for (id, dir, allow_refresh) in jobs {
        let result = async {
            let token =
                credentials::ensure_token(&state.client, &dir, allow_refresh, &state.token_lock)
                    .await
                    .map_err(Failure::Cred)?;
            api::fetch_usage(&state.client, &token)
                .await
                .map_err(Failure::Fetch)
        }
        .await;

        let now = Utc::now();
        let mut inner = state.lock();
        let record = inner.config.settings.record_history;
        let rt = inner.runtime.entry(id.clone()).or_default();
        match result {
            Ok(usage) => {
                let point = Point {
                    t: now.timestamp_millis(),
                    s: usage.session.as_ref().map(|w| w.percent_at(now)),
                    w: usage.weekly.as_ref().map(|w| w.percent),
                };
                rt.usage = Some(usage);
                rt.last_success = Some(now);
                rt.failures = 0;
                rt.auth_error = false;
                drop(inner);
                if record {
                    if let Err(err) = state.history.append(&id, &point) {
                        eprintln!("履歴を書けませんでした: {err}");
                    }
                }
            }
            Err(Failure::Cred(err)) => {
                rt.failures += 1;
                rt.auth_error = matches!(
                    err,
                    CredError::Expired | CredError::Missing | CredError::Invalid
                );
                eprintln!("{id}: 認証情報を使えませんでした: {err:?}");
            }
            Err(Failure::Fetch(err)) => {
                rt.failures += 1;
                match &err {
                    api::FetchError::Unauthorized => rt.auth_error = true,
                    api::FetchError::RateLimited { retry_after_secs } => {
                        let wait =
                            Duration::from_secs(retry_after_secs.unwrap_or(0)).max(interval * 2);
                        rt.blocked_until = Some(Instant::now() + wait);
                    }
                    _ => {}
                }
                eprintln!("{id}: 使用量を取得できませんでした: {err:?}");
            }
        }
    }

    if fetch_status {
        let status = api::fetch_status(&state.client).await;
        let mut inner = state.lock();
        inner.status = Some(status);
        inner.status_fetched = Some(Instant::now());
    }
}
