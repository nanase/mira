//! 使用量 API と稼働状況 API。

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const STATUS_URL: &str = "https://status.claude.com/api/v2/status.json";

#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    pub percent: f64,
    pub resets_at: Option<DateTime<Utc>>,
}

impl Window {
    /// リセット時刻を過ぎていれば、窓が終わって使用率は 0 に戻っている。
    pub fn expired(&self, now: DateTime<Utc>) -> bool {
        self.resets_at.is_some_and(|r| r <= now)
    }

    pub fn percent_at(&self, now: DateTime<Utc>) -> f64 {
        if self.expired(now) {
            0.0
        } else {
            self.percent
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Money {
    pub amount: f64,
    pub currency: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Extra {
    pub used: Money,
    pub limit: Money,
    pub percent: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Usage {
    pub session: Option<Window>,
    pub weekly: Option<Window>,
    pub extra: Option<Extra>,
    pub balance: Option<Money>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    Unauthorized,
    RateLimited { retry_after_secs: Option<u64> },
    Server(u16),
    Network(String),
    Parse,
}

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("mira/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("HTTP クライアントを作れませんでした")
}

pub async fn fetch_usage(
    client: &reqwest::Client,
    access_token: &str,
) -> Result<Usage, FetchError> {
    let response = client
        .get(USAGE_URL)
        .bearer_auth(access_token)
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|err| FetchError::Network(err.to_string()))?;
    let status = response.status().as_u16();
    match status {
        200 => {}
        401 | 403 => return Err(FetchError::Unauthorized),
        429 => {
            let retry_after_secs = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse().ok());
            return Err(FetchError::RateLimited { retry_after_secs });
        }
        _ => return Err(FetchError::Server(status)),
    }
    let body: Value = response.json().await.map_err(|_| FetchError::Parse)?;
    parse_usage(&body).ok_or(FetchError::Parse)
}

fn number(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().trim_end_matches('%').parse().ok(),
        _ => None,
    }
}

fn time(v: Option<&Value>) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(v?.as_str()?)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

fn legacy_window(body: &Value, key: &str) -> Option<Window> {
    let w = body.get(key)?;
    Some(Window {
        percent: number(w.get("utilization")?)?,
        resets_at: time(w.get("resets_at")),
    })
}

fn minor_units(amount: f64, exponent: i32) -> f64 {
    amount / 10f64.powi(exponent)
}

/// 応答を解析する。`limits[]` を正とし、無ければ旧形式の five_hour / seven_day を使う。
///
/// モデル別の週間制限（weekly_scoped）は読まない。値が実際には入ってこないことが多く、
/// 出すと誤解を招くため。
pub fn parse_usage(body: &Value) -> Option<Usage> {
    if !body.is_object() {
        return None;
    }
    let mut session = None;
    let mut weekly = None;

    for limit in body
        .get("limits")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(percent) = limit.get("percent").and_then(number) else {
            continue;
        };
        let window = Window {
            percent,
            resets_at: time(limit.get("resets_at")),
        };
        match limit.get("kind").and_then(Value::as_str) {
            Some("session") => session = Some(window),
            Some("weekly_all") => weekly = Some(window),
            _ => {}
        }
    }

    session = session.or_else(|| legacy_window(body, "five_hour"));
    weekly = weekly.or_else(|| legacy_window(body, "seven_day"));

    Some(Usage {
        session,
        weekly,
        extra: parse_extra(body),
        balance: parse_balance(body),
    })
}

/// 追加使用量。金額は最小単位（セント）で、`decimal_places` 桁で割る。
fn parse_extra(body: &Value) -> Option<Extra> {
    let extra = body.get("extra_usage")?;
    if extra.get("is_enabled").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    let exponent = extra
        .get("decimal_places")
        .and_then(Value::as_i64)
        .unwrap_or(2) as i32;
    let currency = extra
        .get("currency")
        .and_then(Value::as_str)
        .unwrap_or("USD")
        .to_uppercase();
    let limit = minor_units(number(extra.get("monthly_limit")?)?, exponent);
    if limit <= 0.0 {
        return None;
    }
    let used = minor_units(
        extra.get("used_credits").and_then(number).unwrap_or(0.0),
        exponent,
    );
    let percent = extra
        .get("utilization")
        .and_then(number)
        .unwrap_or(used / limit * 100.0);
    Some(Extra {
        used: Money {
            amount: used,
            currency: currency.clone(),
        },
        limit: Money {
            amount: limit,
            currency,
        },
        percent,
    })
}

/// クレジット残高。`spend.balance` が `{amount_minor, currency, exponent}` の形で来たときだけ使う。
fn parse_balance(body: &Value) -> Option<Money> {
    let balance = body.pointer("/spend/balance")?;
    let amount = number(balance.get("amount_minor")?)?;
    let exponent = balance.get("exponent").and_then(Value::as_i64).unwrap_or(2) as i32;
    let currency = balance
        .get("currency")
        .and_then(Value::as_str)
        .unwrap_or("USD")
        .to_uppercase();
    Some(Money {
        amount: minor_units(amount, exponent),
        currency,
    })
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StatusIndicator {
    None,
    Minor,
    Major,
    Critical,
    Unknown,
}

pub async fn fetch_status(client: &reqwest::Client) -> StatusIndicator {
    let Ok(response) = client
        .get(STATUS_URL)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    else {
        return StatusIndicator::Unknown;
    };
    let Ok(body) = response.json::<Value>().await else {
        return StatusIndicator::Unknown;
    };
    match body.pointer("/status/indicator").and_then(Value::as_str) {
        Some("none") => StatusIndicator::None,
        Some("minor") => StatusIndicator::Minor,
        Some("major") => StatusIndicator::Major,
        Some("critical") => StatusIndicator::Critical,
        _ => StatusIndicator::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "five_hour": {"utilization": 6.0, "resets_at": "2026-09-30T11:50:00.392842+00:00"},
      "seven_day": {"utilization": 76.0, "resets_at": "2026-10-02T02:00:00.392862+00:00"},
      "seven_day_opus": null,
      "extra_usage": {"is_enabled": false, "monthly_limit": null, "used_credits": null},
      "limits": [
        {"kind": "session", "percent": 7, "resets_at": "2026-09-30T11:50:00.392842+00:00", "scope": null},
        {"kind": "weekly_all", "percent": 76, "resets_at": "2026-10-02T02:00:00.392862+00:00", "scope": null},
        {"kind": "weekly_scoped", "percent": 0, "resets_at": "2026-10-02T02:00:00+00:00",
         "scope": {"model": {"id": null, "display_name": "Fable"}}},
        {"kind": "weekly_scoped", "percent": 22, "resets_at": "2026-10-02T02:00:00+00:00",
         "scope": {"model": {"id": null, "display_name": "Opus"}}}
      ],
      "spend": {"used": {"amount_minor": 0, "currency": "USD", "exponent": 2}, "balance": null}
    }"#;

    #[test]
    fn parses_current_response_shape() {
        let usage = parse_usage(&serde_json::from_str(SAMPLE).unwrap()).unwrap();
        assert_eq!(usage.session.unwrap().percent, 7.0);
        assert_eq!(usage.weekly.as_ref().unwrap().percent, 76.0);
        assert_eq!(usage.extra, None);
        assert_eq!(usage.balance, None);
    }

    #[test]
    fn falls_back_to_legacy_fields() {
        let body = serde_json::json!({
            "five_hour": {"utilization": "39%", "resets_at": "2026-09-30T11:50:00+00:00"},
            "seven_day": {"utilization": 62, "resets_at": null},
            "seven_day_sonnet": {"utilization": 8, "resets_at": null}
        });
        let usage = parse_usage(&body).unwrap();
        assert_eq!(usage.session.unwrap().percent, 39.0);
        assert_eq!(usage.weekly.unwrap().resets_at, None);
    }

    #[test]
    fn parses_extra_usage_and_balance_in_minor_units() {
        let body = serde_json::json!({
            "extra_usage": {"is_enabled": true, "monthly_limit": 500, "used_credits": 186,
                            "utilization": 37.2, "currency": "usd", "decimal_places": 2},
            "spend": {"balance": {"amount_minor": 1240, "currency": "USD", "exponent": 2}}
        });
        let usage = parse_usage(&body).unwrap();
        let extra = usage.extra.unwrap();
        assert_eq!(extra.used.amount, 1.86);
        assert_eq!(extra.limit.amount, 5.0);
        assert_eq!(extra.limit.currency, "USD");
        assert_eq!(extra.percent, 37.2);
        assert_eq!(usage.balance.unwrap().amount, 12.4);
    }
}
