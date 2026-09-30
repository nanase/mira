//! Claude Code の認証情報（`<設定ディレクトリ>\.credentials.json`）の読み取りと更新。
//!
//! 書き込むのは、利用者が承諾したプロファイルでトークンの期限が切れたときだけである。
//! 書き換えるのは `claudeAiOauth` の accessToken・refreshToken・expiresAt の 3 項目に限る。

use crate::config::write_atomic;
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

const REFRESH_URL: &str = "https://platform.claude.com/v1/oauth/token";
/// Claude Code の公開 OAuth クライアント ID。
const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
/// 期限の直前に送ったリクエストが途中で失効しないよう、この秒数だけ早めに期限切れとみなす。
const LEEWAY_MS: i64 = 60_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredError {
    /// ファイルがない
    Missing,
    /// JSON として読めない、またはトークンが入っていない
    Invalid,
    /// 期限切れで、更新できない（承諾がない、または refresh token も切れている）
    Expired,
    /// 更新を試みて失敗した
    RefreshFailed(String),
}

pub fn credentials_path(dir: &Path) -> PathBuf {
    dir.join(".credentials.json")
}

pub fn exists(dir: &Path) -> bool {
    credentials_path(dir).is_file()
}

struct OAuth {
    access_token: String,
    refresh_token: Option<String>,
    expires_at_ms: Option<i64>,
    refresh_expires_at_ms: Option<i64>,
}

fn read_root(dir: &Path) -> Result<(Value, bool), CredError> {
    let text = std::fs::read_to_string(credentials_path(dir)).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => CredError::Missing,
        _ => CredError::Invalid,
    })?;
    let root: Value = serde_json::from_str(&text).map_err(|_| CredError::Invalid)?;
    Ok((root, text.contains('\n')))
}

fn parse_oauth(root: &Value) -> Result<OAuth, CredError> {
    let oauth = root.get("claudeAiOauth").ok_or(CredError::Invalid)?;
    let access_token = oauth
        .get("accessToken")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or(CredError::Invalid)?
        .to_owned();
    Ok(OAuth {
        access_token,
        refresh_token: oauth
            .get("refreshToken")
            .and_then(Value::as_str)
            .map(str::to_owned),
        expires_at_ms: oauth.get("expiresAt").and_then(Value::as_i64),
        refresh_expires_at_ms: oauth.get("refreshTokenExpiresAt").and_then(Value::as_i64),
    })
}

fn is_expiring(expires_at_ms: Option<i64>, now_ms: i64) -> bool {
    expires_at_ms.is_some_and(|exp| exp - LEEWAY_MS <= now_ms)
}

#[derive(Deserialize)]
struct RefreshResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
}

/// 使えるアクセストークンを返す。期限切れで `allow_refresh` なら更新して書き戻す。
///
/// `lock` は同じファイルへの更新が重ならないようにするためのもの。
pub async fn ensure_token(
    client: &reqwest::Client,
    dir: &Path,
    allow_refresh: bool,
    lock: &tokio::sync::Mutex<()>,
) -> Result<String, CredError> {
    let (root, _) = read_root(dir)?;
    let oauth = parse_oauth(&root)?;
    let now = Utc::now().timestamp_millis();
    if !is_expiring(oauth.expires_at_ms, now) {
        return Ok(oauth.access_token);
    }
    if !allow_refresh {
        return Err(CredError::Expired);
    }

    let _guard = lock.lock().await;
    // 待っている間に Claude Code か別の処理が更新しているかもしれないので読み直す
    let (root, _) = read_root(dir)?;
    let oauth = parse_oauth(&root)?;
    let now = Utc::now().timestamp_millis();
    if !is_expiring(oauth.expires_at_ms, now) {
        return Ok(oauth.access_token);
    }
    let refresh_token = oauth.refresh_token.ok_or(CredError::Expired)?;
    if oauth.refresh_expires_at_ms.is_some_and(|exp| exp <= now) {
        return Err(CredError::Expired);
    }

    let response = client
        .post(REFRESH_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
            ("client_id", CLIENT_ID),
        ])
        .send()
        .await
        .map_err(|err| CredError::RefreshFailed(err.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        // 400/401 は refresh token が無効になっている。再ログインが必要。
        return Err(if status.as_u16() == 400 || status.as_u16() == 401 {
            CredError::Expired
        } else {
            CredError::RefreshFailed(format!("HTTP {status}"))
        });
    }
    let refreshed: RefreshResponse = response
        .json()
        .await
        .map_err(|err| CredError::RefreshFailed(err.to_string()))?;

    // 通信中の変更を上書きしないよう、書く直前にもう一度読み、3 項目だけを差し替える
    let (mut root, pretty) = read_root(dir)?;
    let expires_at = Utc::now().timestamp_millis() + refreshed.expires_in * 1000;
    merge_refreshed(
        &mut root,
        &refreshed.access_token,
        refreshed.refresh_token.as_deref(),
        expires_at,
    )?;
    let text = if pretty {
        serde_json::to_string_pretty(&root)
    } else {
        serde_json::to_string(&root)
    }
    .map_err(|_| CredError::Invalid)?;
    write_atomic(&credentials_path(dir), text.as_bytes())
        .map_err(|err| CredError::RefreshFailed(err.to_string()))?;
    Ok(refreshed.access_token)
}

fn merge_refreshed(
    root: &mut Value,
    access_token: &str,
    refresh_token: Option<&str>,
    expires_at_ms: i64,
) -> Result<(), CredError> {
    let oauth = root
        .get_mut("claudeAiOauth")
        .and_then(Value::as_object_mut)
        .ok_or(CredError::Invalid)?;
    oauth.insert("accessToken".into(), access_token.into());
    if let Some(rt) = refresh_token {
        oauth.insert("refreshToken".into(), rt.into());
    }
    oauth.insert("expiresAt".into(), expires_at_ms.into());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_touches_only_three_fields_and_keeps_order() {
        let mut root: Value = serde_json::from_str(
            r#"{"claudeAiOauth":{"accessToken":"a","refreshToken":"r","expiresAt":1,"scopes":["x"],"subscriptionType":"max"},"other":true}"#,
        )
        .unwrap();
        merge_refreshed(&mut root, "a2", Some("r2"), 42).unwrap();
        assert_eq!(
            serde_json::to_string(&root).unwrap(),
            r#"{"claudeAiOauth":{"accessToken":"a2","refreshToken":"r2","expiresAt":42,"scopes":["x"],"subscriptionType":"max"},"other":true}"#
        );
    }

    #[test]
    fn missing_refresh_token_in_response_keeps_old_one() {
        let mut root: Value = serde_json::from_str(
            r#"{"claudeAiOauth":{"accessToken":"a","refreshToken":"r","expiresAt":1}}"#,
        )
        .unwrap();
        merge_refreshed(&mut root, "a2", None, 5).unwrap();
        assert_eq!(root["claudeAiOauth"]["refreshToken"], "r");
    }

    #[test]
    fn expiring_uses_leeway() {
        assert!(is_expiring(Some(1_000_000), 1_000_000 - 30_000));
        assert!(!is_expiring(Some(1_000_000), 1_000_000 - 120_000));
        assert!(!is_expiring(None, 0));
    }
}
