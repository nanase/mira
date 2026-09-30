//! 使用率の履歴。プロファイルごとに JSON Lines で保存し、90 日より古いものは消す。

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const KEEP_DAYS: i64 = 90;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Point {
    /// 取得時刻（Unix ミリ秒）
    pub t: i64,
    /// 5 時間の使用率
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s: Option<f64>,
    /// 週間の使用率
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w: Option<f64>,
}

pub struct History {
    dir: PathBuf,
}

impl History {
    pub fn new(dir: PathBuf) -> History {
        History { dir }
    }

    fn file(&self, profile_id: &str) -> PathBuf {
        // プロファイル ID は UUID なので、そのままファイル名に使える
        self.dir.join(format!("{profile_id}.jsonl"))
    }

    pub fn append(&self, profile_id: &str, point: &Point) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.file(profile_id))?;
        writeln!(file, "{}", serde_json::to_string(point)?)
    }

    pub fn read(&self, profile_id: &str, since_ms: i64) -> Vec<Point> {
        read_points(&self.file(profile_id))
            .into_iter()
            .filter(|p| p.t >= since_ms)
            .collect()
    }

    pub fn remove(&self, profile_id: &str) {
        let _ = std::fs::remove_file(self.file(profile_id));
    }

    /// 保存期間を過ぎた点を消す。
    pub fn prune(&self, now_ms: i64) {
        let cutoff = now_ms - KEEP_DAYS * 86_400_000;
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "jsonl") {
                continue;
            }
            let points = read_points(&path);
            if points.iter().all(|p| p.t >= cutoff) {
                continue;
            }
            let kept: String = points
                .iter()
                .filter(|p| p.t >= cutoff)
                .filter_map(|p| serde_json::to_string(p).ok())
                .map(|line| line + "\n")
                .collect();
            let _ = crate::config::write_atomic(&path, kept.as_bytes());
        }
    }
}

fn read_points(path: &Path) -> Vec<Point> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_read_and_prune() {
        let dir = std::env::temp_dir().join(format!("mira-history-{}", uuid::Uuid::new_v4()));
        let history = History::new(dir.clone());
        let day = 86_400_000;
        let now = 1_000 * day;
        history
            .append(
                "p",
                &Point {
                    t: now - 100 * day,
                    s: Some(1.0),
                    w: None,
                },
            )
            .unwrap();
        history
            .append(
                "p",
                &Point {
                    t: now - day,
                    s: Some(2.0),
                    w: Some(3.0),
                },
            )
            .unwrap();
        assert_eq!(history.read("p", 0).len(), 2);
        assert_eq!(history.read("p", now - 2 * day).len(), 1);
        history.prune(now);
        assert_eq!(
            history.read("p", 0),
            vec![Point {
                t: now - day,
                s: Some(2.0),
                w: Some(3.0)
            }]
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
