//! ポップアップ、設定、承諾ダイアログの各ウィンドウ。

use crate::state::{AppState, RefreshKind};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, Monitor, PhysicalPosition, PhysicalSize, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

pub const POPOVER: &str = "popover";
pub const SETTINGS: &str = "settings";
pub const CONSENT: &str = "consent";

/// ポップアップのウィンドウ幅（CSS px）。カード 320px と影の余白 10px ずつ。
const POPOVER_WIDTH: f64 = 340.0;
const CONSENT_WIDTH: f64 = 540.0;

#[derive(Default)]
pub struct PopoverState {
    pub profile_id: Option<String>,
    /// クリックされたトレイアイコンの位置（物理ピクセル: x, y, 幅, 高さ）
    anchor: Option<(f64, f64, f64, f64)>,
    pub pinned: bool,
    hidden_at: Option<Instant>,
    pending_show: bool,
    /// 固定中でも、次の表示でトレイアイコンの隣へ置き直す
    reposition: bool,
}

#[derive(Default)]
pub struct Popover(pub Mutex<PopoverState>);

impl Popover {
    pub fn lock(&self) -> MutexGuard<'_, PopoverState> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// ポップアップは起動時に隠して作っておき、フォーカスを失ったら隠す。
pub fn setup_popover(app: &AppHandle) {
    let Some(win) = app.get_webview_window(POPOVER) else {
        return;
    };
    let handle = app.clone();
    win.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            // ロックはウィンドウを操作する前に離す。操作中に届くイベントの処理が同じロックを待つと固まる
            let hide = {
                let popover = handle.state::<Popover>();
                let mut p = popover.lock();
                if !p.pinned {
                    p.hidden_at = Some(Instant::now());
                }
                !p.pinned
            };
            if hide {
                if let Some(win) = handle.get_webview_window(POPOVER) {
                    let _ = win.hide();
                }
            }
        }
    });
}

/// トレイアイコンのクリック。表示中なら隠し、そうでなければそのプロファイルで開く。
///
/// `rect` が None（メニューから開いたとき）は、トレイアイコンの今の位置を取り直して使う。
pub fn toggle_popover(app: &AppHandle, profile_id: &str, rect: Option<tauri::Rect>) {
    let Some(win) = app.get_webview_window(POPOVER) else {
        return;
    };
    let visible = win.is_visible().unwrap_or(false);
    {
        let popover = app.state::<Popover>();
        let mut p = popover.lock();
        let same = p.profile_id.as_deref() == Some(profile_id);
        if visible && same && !p.pinned {
            p.hidden_at = Some(Instant::now());
            drop(p);
            let _ = win.hide();
            return;
        }
        // このクリックでフォーカスが外れて隠れた直後なら、開き直さない
        if !visible
            && same
            && p.hidden_at
                .is_some_and(|t| t.elapsed() < Duration::from_millis(350))
        {
            return;
        }
    }
    open_popover(app, profile_id, rect, false);
}

/// ポップアップをトレイアイコンの隣へ戻して表示する。固定は解かない。
/// 設定画面が開いていれば、トレイアイコンのあるディスプレイの中央へ移す。
pub fn reset_popover_position(app: &AppHandle, profile_id: &str) {
    center_settings_on_tray(app, profile_id);
    open_popover(app, profile_id, None, true);
}

/// 最小化中は動かさない。元に戻すとフォーカスが移り、固定していないポップアップが隠れるため
fn center_settings_on_tray(app: &AppHandle, profile_id: &str) {
    let Some(win) = app.get_webview_window(SETTINGS) else {
        return;
    };
    if win.is_minimized().unwrap_or(false) {
        return;
    }
    let anchor = tray_rect(app, profile_id).map(to_anchor);
    if let Some(monitor) = tray_monitor(app, anchor) {
        center_on(&win, &monitor);
    }
}

/// ウィンドウをディスプレイの作業領域の中央に置く。
///
/// 先に作業領域の左上へ移してから大きさを測る。拡大率の違うディスプレイへ移ると、Windows が大きさを変えるため
fn center_on(win: &WebviewWindow, monitor: &Monitor) {
    let area = monitor.work_area();
    let _ = win.set_position(area.position);
    let Ok(size) = win.outer_size() else {
        return;
    };
    let center = |start: i32, len: u32, win: u32| start + (len as i32 - win as i32).max(0) / 2;
    let _ = win.set_position(PhysicalPosition::new(
        center(area.position.x, area.size.width, size.width),
        center(area.position.y, area.size.height, size.height),
    ));
}

/// 設定画面を出すディスプレイ。ポップアップが出ていればその画面、なければトレイアイコンのある画面にする。
fn settings_monitor(app: &AppHandle) -> Option<Monitor> {
    let popover = app.get_webview_window(POPOVER);
    if let Some(win) = popover.filter(|w| w.is_visible().unwrap_or(false)) {
        if let Ok(Some(monitor)) = win.current_monitor() {
            return Some(monitor);
        }
    }
    let profile_id = app.state::<Popover>().lock().profile_id.clone();
    let rect = profile_id
        .and_then(|id| tray_rect(app, &id))
        .or_else(|| crate::tray::first_rect(app));
    tray_monitor(app, rect.map(to_anchor))
}

/// トレイアイコンの今の位置。隠れているアイコンの一覧が閉じていると取れない。
fn tray_rect(app: &AppHandle, profile_id: &str) -> Option<tauri::Rect> {
    app.tray_by_id(crate::tray::tray_id(profile_id).as_str())
        .and_then(|tray| tray.rect().ok().flatten())
}

fn to_anchor(rect: tauri::Rect) -> (f64, f64, f64, f64) {
    let pos = rect.position.to_physical::<f64>(1.0);
    let size = rect.size.to_physical::<f64>(1.0);
    (pos.x, pos.y, size.width, size.height)
}

fn open_popover(app: &AppHandle, profile_id: &str, rect: Option<tauri::Rect>, reposition: bool) {
    let Some(win) = app.get_webview_window(POPOVER) else {
        return;
    };
    let visible = win.is_visible().unwrap_or(false);
    // ディスプレイの構成が変わると前回の位置は当てにならないので、取り直せなければ捨てる
    let anchor = rect.or_else(|| tray_rect(app, profile_id)).map(to_anchor);
    {
        let popover = app.state::<Popover>();
        let mut p = popover.lock();
        p.profile_id = Some(profile_id.to_owned());
        p.anchor = anchor;
        p.pending_show = !visible;
        p.reposition |= reposition;
    }
    app.state::<AppState>().request_refresh(RefreshKind::Soft);
    let _ = win.emit("popover-open", profile_id);

    // 画面側から大きさの通知が来なければ、そのままの大きさで出す
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(800)).await;
        let pending = {
            let popover = handle.state::<Popover>();
            let p = popover.lock();
            p.pending_show || p.reposition
        };
        if pending {
            fit_popover(&handle, 560.0);
        }
    });
}

/// 画面側で測った内容の高さ（CSS px）に合わせて、大きさと位置を決めて表示する。
pub fn fit_popover(app: &AppHandle, height: f64) {
    let Some(win) = app.get_webview_window(POPOVER) else {
        return;
    };
    let visible = win.is_visible().unwrap_or(false);
    // 状態を写し取ってからロックを離し、そのあとでウィンドウを操作する
    let (pinned, anchor, show, reposition) = {
        let popover = app.state::<Popover>();
        let mut p = popover.lock();
        let show = std::mem::take(&mut p.pending_show);
        let reposition = std::mem::take(&mut p.reposition);
        (p.pinned, p.anchor, show, reposition)
    };

    if pinned && visible && !reposition {
        let _ = win.set_size(LogicalSize::new(POPOVER_WIDTH, height));
        return;
    }

    if let Some(monitor) = tray_monitor(app, anchor) {
        let area = monitor.work_area();
        let work = (
            area.position.x,
            area.position.y,
            area.size.width as i32,
            area.size.height as i32,
        );
        let (x, y, w, h) = place(anchor, work, monitor.scale_factor(), POPOVER_WIDTH, height);
        // 先に移す。拡大率の違うディスプレイへ移ると Windows が大きさを変えるので、そのあとで大きさを決める
        let _ = win.set_position(PhysicalPosition::new(x, y));
        let _ = win.set_size(PhysicalSize::new(w, h));
        let _ = win.set_position(PhysicalPosition::new(x, y));
    } else {
        let _ = win.set_size(LogicalSize::new(POPOVER_WIDTH, height));
    }
    if show {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// トレイアイコンのあるディスプレイ。
///
/// アイコンの位置がわからなければ、カーソルの位置で代える。メニューを選んだ直後なら、カーソルはトレイの近くにある。
/// 主ディスプレイにタスクバーがあるとは限らないので、主ディスプレイは最後の手段にする。
fn tray_monitor(app: &AppHandle, anchor: Option<(f64, f64, f64, f64)>) -> Option<Monitor> {
    anchor
        .and_then(|(x, y, w, h)| {
            app.monitor_from_point(x + w / 2.0, y + h / 2.0)
                .ok()
                .flatten()
        })
        .or_else(|| {
            let cursor = app.cursor_position().ok()?;
            app.monitor_from_point(cursor.x, cursor.y).ok().flatten()
        })
        .or_else(|| app.primary_monitor().ok().flatten())
}

/// ポップアップの位置と大きさ（物理ピクセル）。タスクバーの向きに合わせてアイコンの隣に出す。
///
/// `work` は作業領域（x, y, 幅, 高さ）。タスクバーはこの外側にある。
pub fn place(
    anchor: Option<(f64, f64, f64, f64)>,
    work: (i32, i32, i32, i32),
    scale: f64,
    width: f64,
    height: f64,
) -> (i32, i32, u32, u32) {
    let (left, top, ww, wh) = work;
    let (right, bottom) = (left + ww, top + wh);
    let w = ((width * scale).round() as i32).min(ww);
    let h = ((height * scale).round() as i32).min(wh);
    let clamp_x = |x: i32| x.clamp(left, right - w);
    let clamp_y = |y: i32| y.clamp(top, bottom - h);

    let (x, y) = match anchor {
        None => (right - w, bottom - h),
        Some((ax, ay, aw, ah)) => {
            let (cx, cy) = ((ax + aw / 2.0) as i32, (ay + ah / 2.0) as i32);
            if cy >= bottom {
                (clamp_x(cx - w / 2), bottom - h)
            } else if cy < top {
                (clamp_x(cx - w / 2), top)
            } else if cx < left {
                (left, clamp_y(cy - h / 2))
            } else if cx >= right {
                (right - w, clamp_y(cy - h / 2))
            } else {
                // 隠れているアイコンの一覧（作業領域の中）から開いたとき
                let above = ay as i32 - h;
                (
                    clamp_x(cx - w / 2),
                    if above >= top {
                        above
                    } else {
                        clamp_y((ay + ah) as i32)
                    },
                )
            }
        }
    };
    (x, y, w as u32, h as u32)
}

pub fn set_pinned(app: &AppHandle, pinned: bool) {
    app.state::<Popover>().lock().pinned = pinned;
    let _ = app.emit_to(POPOVER, "popover-pinned", pinned);
}

pub fn hide_popover(app: &AppHandle) {
    {
        let popover = app.state::<Popover>();
        let mut p = popover.lock();
        p.pinned = false;
        p.hidden_at = Some(Instant::now());
    }
    if let Some(win) = app.get_webview_window(POPOVER) {
        let _ = win.hide();
    }
}

/// 設定のテーマをタイトルバーに反映する。None は Windows の設定に従う。
pub fn window_theme(app: &AppHandle) -> Option<tauri::Theme> {
    match app.state::<AppState>().lock().config.settings.theme {
        crate::config::Theme::System => None,
        crate::config::Theme::Dark => Some(tauri::Theme::Dark),
        crate::config::Theme::Light => Some(tauri::Theme::Light),
    }
}

pub fn apply_theme(app: &AppHandle) {
    let theme = window_theme(app);
    for label in [SETTINGS, CONSENT, POPOVER] {
        if let Some(win) = app.get_webview_window(label) {
            let _ = win.set_theme(theme);
        }
    }
}

pub fn open_settings(app: &AppHandle) {
    // ポップアップを隠す前に決める
    let monitor = app
        .get_webview_window(SETTINGS)
        .is_none()
        .then(|| settings_monitor(app))
        .flatten();
    if !app.state::<Popover>().lock().pinned {
        if let Some(win) = app.get_webview_window(POPOVER) {
            let _ = win.hide();
        }
    }
    if let Some(win) = app.get_webview_window(SETTINGS) {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let title = match app.state::<AppState>().lang {
        crate::i18n::Lang::Ja => "Mira の設定",
        crate::i18n::Lang::En => "Mira Settings",
    };
    let theme = window_theme(app);
    let handle = app.clone();
    // Windows では、メニューのイベントや同期コマンドの中（メインスレッド）でウィンドウを作ると
    // WebView2 の初期化を待ったまま固まる。別のスレッドで作る
    tauri::async_runtime::spawn(async move {
        let result =
            WebviewWindowBuilder::new(&handle, SETTINGS, WebviewUrl::App("settings.html".into()))
                .title(title)
                .inner_size(760.0, 640.0)
                .min_inner_size(640.0, 480.0)
                .theme(theme)
                .visible(false)
                .build();
        match result {
            Ok(win) => {
                match &monitor {
                    Some(monitor) => center_on(&win, monitor),
                    None => {
                        let _ = win.center();
                    }
                }
                let _ = win.show();
                let _ = win.set_focus();
            }
            Err(err) => eprintln!("設定画面を開けませんでした: {err}"),
        }
    });
}

pub fn open_consent(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(CONSENT) {
        let _ = win.emit("consent-changed", ());
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let theme = window_theme(app);
    let handle = app.clone();
    // 作る場所の注意は open_settings と同じ
    tauri::async_runtime::spawn(async move {
        let result =
            WebviewWindowBuilder::new(&handle, CONSENT, WebviewUrl::App("consent.html".into()))
                .title("Mira")
                .inner_size(CONSENT_WIDTH, 480.0)
                .resizable(false)
                .maximizable(false)
                .minimizable(false)
                .decorations(false)
                // 影を付けると Windows 10 では高さがタイトルバー分ずれるので、枠は CSS で描く
                .shadow(false)
                .always_on_top(true)
                .skip_taskbar(false)
                .visible(false)
                .theme(theme)
                .center()
                .build();
        if let Err(err) = result {
            eprintln!("確認ダイアログを開けませんでした: {err}");
        }
    });
}

/// 承諾ダイアログの内容に合わせて高さを変え、画面中央に出す。
pub fn fit_consent(app: &AppHandle, height: f64) {
    let Some(win) = app.get_webview_window(CONSENT) else {
        return;
    };
    let _ = win.set_size(LogicalSize::new(CONSENT_WIDTH, height));
    let _ = win.center();
    let _ = win.show();
    let _ = win.set_focus();
}

pub fn close_consent(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(CONSENT) {
        let _ = win.destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::place;

    const WORK: (i32, i32, i32, i32) = (0, 0, 1920, 1040);

    #[test]
    fn bottom_taskbar_opens_above_icon() {
        let (x, y, w, h) = place(Some((1700.0, 1045.0, 24.0, 30.0)), WORK, 1.0, 340.0, 500.0);
        assert_eq!((w, h), (340, 500));
        assert_eq!(y, 540);
        assert_eq!(x, 1712 - 170);
    }

    #[test]
    fn stays_inside_work_area_near_the_edge() {
        let (x, _, w, _) = place(Some((1890.0, 1045.0, 24.0, 30.0)), WORK, 1.5, 340.0, 500.0);
        assert_eq!(x + w as i32, 1920);
    }

    #[test]
    fn top_and_side_taskbars() {
        let work = (0, 40, 1920, 1040);
        assert_eq!(
            place(Some((1700.0, 5.0, 24.0, 30.0)), work, 1.0, 340.0, 500.0).1,
            40
        );
        let work = (60, 0, 1860, 1080);
        let (x, y, _, _) = place(Some((10.0, 900.0, 24.0, 24.0)), work, 1.0, 340.0, 500.0);
        assert_eq!((x, y), (60, 580));
    }

    #[test]
    fn overflow_area_opens_above_the_icon() {
        let (_, y, _, h) = place(Some((1600.0, 950.0, 32.0, 32.0)), WORK, 1.0, 340.0, 400.0);
        assert_eq!(y + h as i32, 950);
    }
}
