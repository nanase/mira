//! 使用率の色分けとペースの判定。元アプリ（Claude-Usage-Tracker）の
//! UsageStatusCalculator と PaceStatus に合わせている。

use chrono::{DateTime, Utc};

pub const SESSION_SECS: f64 = 5.0 * 3600.0;
pub const WEEK_SECS: f64 = 7.0 * 86400.0;

/// 窓の経過率（0〜1）。リセット時刻が過ぎていれば None。
pub fn elapsed_fraction(
    resets_at: Option<DateTime<Utc>>,
    duration_secs: f64,
    now: DateTime<Utc>,
) -> Option<f64> {
    let reset = resets_at?;
    if reset <= now || duration_secs <= 0.0 {
        return None;
    }
    let remaining = (reset - now).num_milliseconds() as f64 / 1000.0;
    Some(((duration_secs - remaining) / duration_secs).clamp(0.0, 1.0))
}

/// バーと数値の色。0 = 緑、1 = 橙、2 = 赤。
///
/// 経過率が 15% 以上なら、期末の予測使用率で判定する。
/// それより前は使用率（残り表示なら残り率）そのもので判定する。
pub fn level(used: f64, elapsed: Option<f64>, show_remaining: bool) -> u8 {
    let u = used / 100.0;
    if let Some(t) = elapsed {
        if (0.15..1.0).contains(&t) && u > 0.0 {
            let projected = u / t;
            return if projected < 0.7 {
                0
            } else if projected < 0.9 {
                1
            } else {
                2
            };
        }
    }
    if show_remaining {
        let remaining = (100.0 - used).max(0.0);
        if remaining >= 30.0 {
            0
        } else if remaining >= 10.0 {
            1
        } else {
            2
        }
    } else if used < 70.0 {
        0
    } else if used < 90.0 {
        1
    } else {
        2
    }
}

/// 時間マーカーの 6 段階の色。0 = 余裕 … 5 = 大幅超過。
/// 経過率が 3% 未満、または窓が終わっているときは判定しない。
pub fn pace(used: f64, elapsed: Option<f64>) -> Option<u8> {
    let t = elapsed?;
    if !(0.03..1.0).contains(&t) {
        return None;
    }
    if used <= 0.0 {
        return Some(0);
    }
    let projected = (used / 100.0) / t;
    Some(if projected < 0.5 {
        0
    } else if projected < 0.75 {
        1
    } else if projected < 0.9 {
        2
    } else if projected < 1.0 {
        3
    } else if projected < 1.2 {
        4
    } else {
        5
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn elapsed_fraction_counts_from_window_start() {
        let now = Utc::now();
        let reset = now + Duration::minutes(130);
        let t = elapsed_fraction(Some(reset), SESSION_SECS, now).unwrap();
        assert!((t - 170.0 / 300.0).abs() < 1e-3);
        assert_eq!(
            elapsed_fraction(Some(now - Duration::minutes(1)), SESSION_SECS, now),
            None
        );
        assert_eq!(elapsed_fraction(None, SESSION_SECS, now), None);
    }

    #[test]
    fn level_uses_projection_after_15_percent() {
        assert_eq!(level(39.0, Some(0.567), false), 0);
        assert_eq!(level(62.0, Some(0.72), false), 1);
        assert_eq!(level(62.0, Some(0.679), false), 2);
    }

    #[test]
    fn level_falls_back_to_thresholds() {
        assert_eq!(level(69.0, Some(0.1), false), 0);
        assert_eq!(level(75.0, None, false), 1);
        assert_eq!(level(95.0, None, false), 2);
        assert_eq!(level(75.0, None, true), 1);
        assert_eq!(level(50.0, None, true), 0);
        assert_eq!(level(92.0, None, true), 2);
    }

    #[test]
    fn pace_tiers_match_design_samples() {
        let tiers: Vec<_> = [20.0, 31.0, 41.0, 47.0, 55.0, 70.0]
            .iter()
            .map(|u| pace(*u, Some(0.5)).unwrap())
            .collect();
        assert_eq!(tiers, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(pace(10.0, Some(0.02)), None);
        assert_eq!(pace(0.0, Some(0.5)), Some(0));
    }
}
