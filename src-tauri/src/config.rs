//! 設定とプロファイルの保存。アプリの設定ディレクトリの config.json に置く。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IconStyle {
    Ring,
    /// 以前の「リング＋時間マーカー」。読み込み時に Ring と tray_marker に置き換える
    RingTick,
    Double,
    Number,
    Bars,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ColorMode {
    Color,
    Mono,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MarkerMode {
    White,
    Pace,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    System,
    Dark,
    Light,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub refresh_minutes: u32,
    pub icon_style: IconStyle,
    /// トレイアイコンに時間マーカーを描くか
    pub tray_marker: bool,
    pub color_mode: ColorMode,
    pub marker_mode: MarkerMode,
    pub show_remaining: bool,
    pub theme: Theme,
    pub autostart: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            refresh_minutes: 5,
            icon_style: IconStyle::Ring,
            tray_marker: false,
            color_mode: ColorMode::Color,
            marker_mode: MarkerMode::White,
            show_remaining: false,
            theme: Theme::System,
            autostart: false,
        }
    }
}

impl Settings {
    pub const MIN_REFRESH_MINUTES: u32 = 1;
    pub const MAX_REFRESH_MINUTES: u32 = 30;

    pub fn normalized(mut self) -> Self {
        self.refresh_minutes = self
            .refresh_minutes
            .clamp(Self::MIN_REFRESH_MINUTES, Self::MAX_REFRESH_MINUTES);
        if self.icon_style == IconStyle::RingTick {
            self.icon_style = IconStyle::Ring;
            self.tray_marker = true;
        }
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub dir: PathBuf,
    /// トークンの書き戻しへの承諾。None はまだ尋ねていない。
    #[serde(default)]
    pub allow_token_refresh: Option<bool>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub settings: Settings,
    pub profiles: Vec<Profile>,
    /// 登録を断られたディレクトリ。自動検出で二度と尋ねない。
    pub declined_dirs: Vec<PathBuf>,
}

impl Config {
    pub fn load(path: &Path) -> Config {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Config::default();
        };
        match serde_json::from_str::<Config>(&text) {
            Ok(mut config) => {
                config.settings = config.settings.normalized();
                config
            }
            Err(err) => {
                // 読めない設定で上書きしないよう、退避してから既定値で始める
                eprintln!("config.json を読めませんでした: {err}");
                let _ = std::fs::rename(path, path.with_extension("json.broken"));
                Config::default()
            }
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self)?;
        write_atomic(path, text.as_bytes())
    }

    pub fn profile(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    pub fn profile_mut(&mut self, id: &str) -> Option<&mut Profile> {
        self.profiles.iter_mut().find(|p| p.id == id)
    }

    pub fn knows_dir(&self, dir: &Path) -> bool {
        self.profiles.iter().any(|p| same_dir(&p.dir, dir))
            || self.declined_dirs.iter().any(|d| same_dir(d, dir))
    }
}

/// Windows のパスは大文字小文字を区別しないので、比較用に正規化する。
pub fn same_dir(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| {
        p.to_string_lossy()
            .trim_end_matches(['\\', '/'])
            .replace('/', "\\")
            .to_lowercase()
    };
    norm(a) == norm(b)
}

/// 同じディレクトリに一時ファイルを書いてから置き換える。
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".mira-tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_fields_and_missing_fields_are_tolerated() {
        let config: Config =
            serde_json::from_str(r#"{"settings":{"refreshMinutes":99,"future":1},"profiles":[]}"#)
                .unwrap();
        assert_eq!(config.settings.clone().normalized().refresh_minutes, 30);
        assert_eq!(config.settings.icon_style, IconStyle::Ring);
    }

    #[test]
    fn old_ring_tick_style_becomes_ring_with_marker() {
        let settings: Settings = serde_json::from_str(r#"{"iconStyle":"ringTick"}"#).unwrap();
        let settings = settings.normalized();
        assert_eq!(settings.icon_style, IconStyle::Ring);
        assert!(settings.tray_marker);
    }

    #[test]
    fn dirs_compare_case_insensitively() {
        assert!(same_dir(
            Path::new(r"C:\Users\A\.claude\"),
            Path::new("c:/users/a/.CLAUDE")
        ));
        assert!(!same_dir(
            Path::new(r"C:\a\.claude"),
            Path::new(r"C:\a\.claude-work")
        ));
    }
}
