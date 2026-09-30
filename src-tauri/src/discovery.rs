//! Claude Code の設定ディレクトリを探す。見つけるだけで、中身には触れない。

use crate::credentials;
use std::path::{Path, PathBuf};

/// 認証情報を持つ設定ディレクトリの候補。
///
/// - 環境変数 `CLAUDE_CONFIG_DIR`
/// - ホーム直下の `.claude` と、`.claude` で始まるディレクトリ（`.claude-work` など）
pub fn candidates(home: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    let mut push = |dir: PathBuf| {
        if credentials::exists(&dir) && !found.iter().any(|d| crate::config::same_dir(d, &dir)) {
            found.push(dir);
        }
    };
    if let Some(dir) = std::env::var_os("CLAUDE_CONFIG_DIR") {
        push(PathBuf::from(dir));
    }
    push(home.join(".claude"));
    if let Ok(entries) = std::fs::read_dir(home) {
        let mut dirs: Vec<PathBuf> = entries
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .starts_with(".claude")
            })
            .map(|e| e.path())
            .collect();
        dirs.sort();
        for dir in dirs {
            push(dir);
        }
    }
    found
}

/// ホームを `%USERPROFILE%` に置き換えた表示用のパス。
pub fn display_path(path: &Path, home: &Path) -> String {
    let p = path.to_string_lossy().to_string();
    let h = home
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_string();
    if !h.is_empty() && p.to_lowercase().starts_with(&h.to_lowercase()) {
        let rest = &p[h.len()..];
        if rest.is_empty() || rest.starts_with(['\\', '/']) {
            return format!("%USERPROFILE%{}", rest.replace('/', "\\"));
        }
    }
    p
}

/// 登録時の表示名の初期値。`.claude` は main、`.claude-work` は work とする。
pub fn default_name(dir: &Path) -> String {
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let lower = name.to_lowercase();
    if lower == ".claude" {
        return "main".into();
    }
    if let Some(rest) = lower.strip_prefix(".claude") {
        let rest = rest.trim_start_matches(['-', '_', '.']);
        if !rest.is_empty() {
            return name[name.len() - rest.len()..].to_string();
        }
    }
    let trimmed = name.trim_start_matches('.');
    if trimmed.is_empty() {
        "main".into()
    } else {
        trimmed.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_names() {
        assert_eq!(default_name(Path::new(r"C:\u\.claude")), "main");
        assert_eq!(default_name(Path::new(r"C:\u\.claude-Work")), "Work");
        assert_eq!(default_name(Path::new(r"D:\configs\lab")), "lab");
    }

    #[test]
    fn display_path_hides_home() {
        let home = Path::new(r"C:\Users\someone");
        assert_eq!(
            display_path(Path::new(r"C:\Users\someone\.claude"), home),
            r"%USERPROFILE%\.claude"
        );
        assert_eq!(
            display_path(Path::new(r"C:\Users\someone2\.claude"), home),
            r"C:\Users\someone2\.claude"
        );
        assert_eq!(display_path(Path::new(r"D:\x"), home), r"D:\x");
    }
}
