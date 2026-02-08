use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentEntry {
    pub key: String,
    pub last_run: u64, // Unix timestamp (milliseconds)
    pub count: u32,
}

/// Maximum number of recent entries to keep
const MAX_RECENTS: usize = 100;

/// Loads recent script executions from the config directory.
/// Returns an empty Vec if the file doesn't exist or is corrupted.
///
/// # Arguments
/// * `config_dir` - Path to the config directory
///
/// # Returns
/// A Vec of RecentEntry structs
pub fn load_recents(config_dir: &Path) -> Vec<RecentEntry> {
    let path = config_dir.join("recents.json");

    if !path.exists() {
        return Vec::new();
    }

    match std::fs::read_to_string(&path) {
        Ok(contents) => {
            serde_json::from_str::<Vec<RecentEntry>>(&contents).unwrap_or_else(|_| Vec::new())
        }
        Err(_) => Vec::new(),
    }
}

/// Saves recent script executions to the config directory.
///
/// # Arguments
/// * `config_dir` - Path to the config directory
/// * `recents` - Slice of RecentEntry structs
pub fn save_recents(config_dir: &Path, recents: &[RecentEntry]) {
    let path = config_dir.join("recents.json");
    let json = serde_json::to_string_pretty(&recents).unwrap_or_else(|_| "[]".to_string());
    std::fs::write(&path, json).ok();
}

/// Records a script execution, updating existing entry or creating a new one.
/// Evicts the lowest-frecency entry if the list exceeds MAX_RECENTS.
///
/// # Arguments
/// * `recents` - Mutable reference to the recents Vec
/// * `key` - The script key that was executed
pub fn record_execution(recents: &mut Vec<RecentEntry>, key: &str) {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // Find existing entry and update it
    if let Some(entry) = recents.iter_mut().find(|e| e.key == key) {
        entry.count += 1;
        entry.last_run = now_ms;
    } else {
        // Create new entry
        recents.push(RecentEntry {
            key: key.to_string(),
            last_run: now_ms,
            count: 1,
        });
    }

    // Evict lowest-frecency entry if over limit
    if recents.len() > MAX_RECENTS {
        if let Some((min_idx, _)) = recents.iter().enumerate().min_by(|(_, a), (_, b)| {
            let score_a = frecency_score(a.count, a.last_run, now_ms);
            let score_b = frecency_score(b.count, b.last_run, now_ms);
            score_a
                .partial_cmp(&score_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            recents.remove(min_idx);
        }
    }
}

/// Calculates a frecency (frequency + recency) score for a recent entry.
/// Higher scores indicate more frequently and recently used scripts.
///
/// # Arguments
/// * `count` - Number of times the script has been executed
/// * `last_run_ms` - Unix timestamp (milliseconds) of the last execution
/// * `now_ms` - Current time in milliseconds since UNIX epoch
///
/// # Returns
/// A frecency score (higher is better)
pub fn frecency_score(count: u32, last_run_ms: u64, now_ms: u64) -> f64 {
    let age_in_days = (now_ms.saturating_sub(last_run_ms)) as f64 / (1000.0 * 60.0 * 60.0 * 24.0);
    let halflife = 14.0;
    let frequency_score = ((count + 1) as f64).log2() + 1.0;
    frequency_score * (0.5_f64).powf(age_in_days / halflife)
}

/// Returns the current time in milliseconds since UNIX epoch.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_recents_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let recents = load_recents(temp_dir.path());
        assert!(recents.is_empty());
    }

    #[test]
    fn test_save_and_load_recents() {
        let temp_dir = TempDir::new().unwrap();
        let recents = vec![
            RecentEntry {
                key: "a1b2c3d4:root:dev".to_string(),
                last_run: 1000000,
                count: 5,
            },
            RecentEntry {
                key: "a1b2c3d4:root:build".to_string(),
                last_run: 2000000,
                count: 3,
            },
        ];

        save_recents(temp_dir.path(), &recents);
        let loaded = load_recents(temp_dir.path());

        assert_eq!(recents, loaded);
    }

    #[test]
    fn test_load_recents_corrupted_json() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("recents.json");
        fs::write(&path, "not valid json").unwrap();

        let recents = load_recents(temp_dir.path());
        assert!(recents.is_empty());
    }

    #[test]
    fn test_record_execution_creates_new_entry() {
        let mut recents = Vec::new();
        record_execution(&mut recents, "a1b2c3d4:root:dev");

        assert_eq!(recents.len(), 1);
        assert_eq!(recents[0].key, "a1b2c3d4:root:dev");
        assert_eq!(recents[0].count, 1);
        assert!(recents[0].last_run > 0);
    }

    #[test]
    fn test_record_execution_updates_existing_entry() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let mut recents = vec![RecentEntry {
            key: "a1b2c3d4:root:dev".to_string(),
            last_run: now - 10000,
            count: 5,
        }];

        record_execution(&mut recents, "a1b2c3d4:root:dev");

        assert_eq!(recents.len(), 1);
        assert_eq!(recents[0].count, 6);
        assert!(recents[0].last_run > now - 10000);
    }

    #[test]
    fn test_record_execution_evicts_at_limit() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Create 100 entries
        let mut recents: Vec<RecentEntry> = (0..100)
            .map(|i| RecentEntry {
                key: format!("key_{}", i),
                last_run: now - (i as u64 * 1000),
                count: i as u32 + 1,
            })
            .collect();

        // Add one more, should evict the lowest-frecency entry
        record_execution(&mut recents, "new_key");

        assert_eq!(recents.len(), 100);
        assert!(recents.iter().any(|e| e.key == "new_key"));
    }

    #[test]
    fn test_frecency_score_higher_count_increases_score() {
        let now = now_ms();

        let score1 = frecency_score(1, now, now);
        let score10 = frecency_score(10, now, now);

        assert!(score10 > score1);
    }

    #[test]
    fn test_frecency_score_recent_has_higher_score() {
        let now = now_ms();

        let recent = frecency_score(5, now, now);
        let old = frecency_score(5, now - (30 * 24 * 60 * 60 * 1000), now); // 30 days ago

        assert!(recent > old);
    }

    #[test]
    fn test_frecency_score_positive() {
        let now = now_ms();

        let score = frecency_score(1, now, now);
        assert!(score > 0.0);
    }

    #[test]
    fn test_frecency_score_zero_count() {
        let now = now_ms();

        // Count of 0 should still produce a positive score due to +1 in formula
        let score = frecency_score(0, now, now);
        assert!(score > 0.0);
    }

    #[test]
    fn test_frecency_score_halflife_decay() {
        let now = now_ms();

        let halflife_ms = 14 * 24 * 60 * 60 * 1000; // 14 days in milliseconds

        let score_now = frecency_score(10, now, now);
        let score_halflife = frecency_score(10, now - halflife_ms, now);

        // After one halflife, score should be approximately half
        let ratio = score_halflife / score_now;
        assert!((ratio - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_save_empty_recents() {
        let temp_dir = TempDir::new().unwrap();
        let recents: Vec<RecentEntry> = Vec::new();

        save_recents(temp_dir.path(), &recents);

        let path = temp_dir.path().join("recents.json");
        assert!(path.exists());

        let loaded = load_recents(temp_dir.path());
        assert!(loaded.is_empty());
    }
}
