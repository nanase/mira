//! Mira: Claude の使用量制限をタスクトレイに表示する。

mod api;
mod commands;
mod config;
mod credentials;
mod discovery;
mod history;
mod i18n;
pub mod icon;
mod pace;
mod platform;
mod state;
mod tray;
mod windows;

use state::AppState;
use std::sync::Mutex;
use tauri::{Manager, RunEvent};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // 動作確認用。デバッグビルドでは 2 つ目の起動に --debug-popover を付けるとポップアップを開く
            #[cfg(debug_assertions)]
            if args.iter().any(|a| a == "--debug-popover") {
                let first = app
                    .state::<AppState>()
                    .lock()
                    .config
                    .profiles
                    .first()
                    .map(|p| p.id.clone());
                if let Some(id) = first {
                    windows::toggle_popover(app, &id, None);
                }
                return;
            }
            let _ = args;
            windows::open_settings(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config_path = app.path().app_config_dir()?.join("config.json");
            let history = history::History::new(app.path().app_local_data_dir()?.join("history"));
            history.prune(chrono::Utc::now().timestamp_millis());
            app.manage(AppState {
                inner: Mutex::new(state::Inner {
                    config: config::Config::load(&config_path),
                    runtime: Default::default(),
                    status: None,
                    status_fetched: None,
                    request: None,
                    consent: Default::default(),
                }),
                config_path,
                home: app.path().home_dir()?,
                history,
                client: api::client(),
                notify: tokio::sync::Notify::new(),
                token_lock: tokio::sync::Mutex::new(()),
                fonts: icon::Fonts::load(),
                lang: i18n::Lang::detect(),
            });
            app.manage(windows::Popover::default());
            app.manage(tray::TrayIds::default());

            let handle = app.handle();
            windows::setup_popover(handle);
            windows::apply_theme(handle);
            tray::sync(handle);
            windows::restore_popover(handle);
            if app.state::<AppState>().queue_consents() {
                windows::open_consent(handle);
            }
            tauri::async_runtime::spawn(state::run(handle.clone()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::refresh_now,
            commands::popover_info,
            commands::select_popover_profile,
            commands::fit_popover,
            commands::set_pinned,
            commands::hide_popover,
            commands::open_settings,
            commands::open_link,
            commands::update_settings,
            commands::rename_profile,
            commands::remove_profile,
            commands::set_token_refresh,
            commands::add_profile_folder,
            commands::rescan_profiles,
            commands::get_consent,
            commands::consent_register,
            commands::consent_decline,
            commands::consent_refresh,
            commands::fit_consent,
            commands::get_history,
            commands::clear_history,
            commands::quit,
        ])
        .build(tauri::generate_context!())
        .expect("Mira を起動できませんでした")
        .run(|_app, event| {
            // 設定画面を閉じてもトレイに残す。終了はメニューの「Mira を終了」から
            if let RunEvent::ExitRequested {
                api, code: None, ..
            } = event
            {
                api.prevent_exit();
            }
        });
}
