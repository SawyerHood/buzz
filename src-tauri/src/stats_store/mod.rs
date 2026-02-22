use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tracing::{debug, info, warn};

const STATS_FILE_NAME: &str = "stats.json";
const DEFAULT_HISTORY_WINDOW_DAYS: usize = 30;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DailyStats {
    #[serde(default)]
    pub transcriptions: u64,
    #[serde(default)]
    pub words: u64,
    #[serde(default)]
    pub recording_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageStats {
    #[serde(default)]
    pub total_transcriptions: u64,
    #[serde(default)]
    pub total_words: u64,
    #[serde(default)]
    pub total_recording_seconds: f64,
    #[serde(default)]
    pub daily_stats: BTreeMap<String, DailyStats>,
    #[serde(default = "today_date_key")]
    pub last_updated: String,
}

impl Default for UsageStats {
    fn default() -> Self {
        Self {
            total_transcriptions: 0,
            total_words: 0,
            total_recording_seconds: 0.0,
            daily_stats: BTreeMap::new(),
            last_updated: today_date_key(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DailyWordCount {
    pub date: String,
    pub words: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageStatsReport {
    pub total_transcriptions: u64,
    pub total_words: u64,
    pub total_recording_seconds: f64,
    pub words_per_minute: f64,
    pub average_transcription_length: f64,
    pub streak_days: u64,
    pub today: DailyStats,
    pub daily_word_history: Vec<DailyWordCount>,
    pub last_updated: String,
}

#[derive(Debug)]
pub struct StatsStore {
    file_path: PathBuf,
    io_lock: Mutex<()>,
}

impl StatsStore {
    pub fn new(app: &AppHandle) -> Result<Self, String> {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("Failed to resolve app data directory: {error}"))?;

        let file_path = app_data_dir.join(STATS_FILE_NAME);
        debug!(path = %file_path.display(), "initializing usage stats store");
        Self::new_with_file_path(file_path)
    }

    pub fn new_with_file_path(file_path: PathBuf) -> Result<Self, String> {
        ensure_stats_file(&file_path)?;
        Ok(Self {
            file_path,
            io_lock: Mutex::new(()),
        })
    }

    pub fn record_transcription(
        &self,
        word_count: u64,
        recording_duration_secs: f64,
    ) -> Result<(), String> {
        let sanitized_duration = sanitize_seconds(recording_duration_secs);
        let today = today_date_key();
        debug!(
            word_count,
            recording_duration_secs = sanitized_duration,
            date = %today,
            "recording usage stats for transcription"
        );

        let _guard = self
            .io_lock
            .lock()
            .map_err(|_| "Stats store lock is poisoned".to_string())?;
        let mut stats = self.read_usage_stats()?;

        stats.total_transcriptions = stats.total_transcriptions.saturating_add(1);
        stats.total_words = stats.total_words.saturating_add(word_count);
        stats.total_recording_seconds =
            sanitize_seconds(stats.total_recording_seconds + sanitized_duration);

        let day_stats = stats.daily_stats.entry(today.clone()).or_default();
        day_stats.transcriptions = day_stats.transcriptions.saturating_add(1);
        day_stats.words = day_stats.words.saturating_add(word_count);
        day_stats.recording_seconds =
            sanitize_seconds(day_stats.recording_seconds + sanitized_duration);

        stats.last_updated = today;
        self.write_usage_stats(&stats)
    }

    pub fn get_usage_stats(&self) -> Result<UsageStatsReport, String> {
        let _guard = self
            .io_lock
            .lock()
            .map_err(|_| "Stats store lock is poisoned".to_string())?;
        let stats = self.read_usage_stats()?;
        Ok(build_usage_report(
            &stats,
            today_local_date(),
            DEFAULT_HISTORY_WINDOW_DAYS,
        ))
    }

    pub fn reset_usage_stats(&self) -> Result<(), String> {
        info!("resetting usage stats");
        let _guard = self
            .io_lock
            .lock()
            .map_err(|_| "Stats store lock is poisoned".to_string())?;
        self.write_usage_stats(&UsageStats::default())
    }

    fn read_usage_stats(&self) -> Result<UsageStats, String> {
        if !self.file_path.exists() {
            return Ok(UsageStats::default());
        }

        let raw_contents = fs::read_to_string(&self.file_path)
            .map_err(|error| format!("Failed to read usage stats file: {error}"))?;
        if raw_contents.trim().is_empty() {
            return Ok(UsageStats::default());
        }

        let mut stats = match serde_json::from_str::<UsageStats>(&raw_contents) {
            Ok(parsed) => parsed,
            Err(error) => {
                self.recover_malformed_stats_file(format!(
                    "Failed to parse usage stats file: {error}"
                ))?;
                return Ok(UsageStats::default());
            }
        };

        normalize_usage_stats(&mut stats);
        Ok(stats)
    }

    fn write_usage_stats(&self, stats: &UsageStats) -> Result<(), String> {
        let serialized = serde_json::to_vec_pretty(stats)
            .map_err(|error| format!("Failed to serialize usage stats: {error}"))?;
        let temp_path = temp_file_path_for(&self.file_path);

        let mut temp_file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)
            .map_err(|error| {
                format!(
                    "Failed to create usage stats temp file `{}`: {error}",
                    temp_path.display()
                )
            })?;

        if let Err(error) = temp_file.write_all(&serialized) {
            let _ = fs::remove_file(&temp_path);
            return Err(format!(
                "Failed to write usage stats temp file `{}`: {error}",
                temp_path.display()
            ));
        }

        if let Err(error) = temp_file.sync_all() {
            let _ = fs::remove_file(&temp_path);
            return Err(format!(
                "Failed to flush usage stats temp file `{}`: {error}",
                temp_path.display()
            ));
        }

        drop(temp_file);

        fs::rename(&temp_path, &self.file_path).map_err(|error| {
            let _ = fs::remove_file(&temp_path);
            format!("Failed to finalize usage stats file: {error}")
        })?;

        Ok(())
    }

    fn recover_malformed_stats_file(&self, reason: String) -> Result<(), String> {
        let backup_path = backup_corrupt_stats_file(&self.file_path)?;
        self.write_usage_stats(&UsageStats::default())?;
        warn!(
            path = %self.file_path.display(),
            backup = %backup_path.display(),
            reason = %reason,
            "recovered malformed usage stats file"
        );
        Ok(())
    }
}

fn ensure_stats_file(file_path: &Path) -> Result<(), String> {
    if let Some(parent_dir) = file_path.parent() {
        fs::create_dir_all(parent_dir)
            .map_err(|error| format!("Failed to create usage stats directory: {error}"))?;
    }

    if !file_path.exists() {
        let initial_stats = serde_json::to_vec_pretty(&UsageStats::default())
            .map_err(|error| format!("Failed to serialize initial usage stats: {error}"))?;
        fs::write(file_path, initial_stats)
            .map_err(|error| format!("Failed to initialize usage stats file: {error}"))?;
        info!(path = %file_path.display(), "created usage stats file");
    }

    Ok(())
}

fn normalize_usage_stats(stats: &mut UsageStats) {
    stats.total_recording_seconds = sanitize_seconds(stats.total_recording_seconds);
    if parse_date_key(&stats.last_updated).is_none() {
        stats.last_updated = today_date_key();
    }

    stats.daily_stats.retain(|date, day_stats| {
        if parse_date_key(date).is_none() {
            return false;
        }

        day_stats.recording_seconds = sanitize_seconds(day_stats.recording_seconds);
        true
    });
}

fn build_usage_report(
    stats: &UsageStats,
    today: NaiveDate,
    history_days: usize,
) -> UsageStatsReport {
    let today_key = date_key(today);
    let today_stats = stats
        .daily_stats
        .get(&today_key)
        .cloned()
        .unwrap_or_default();
    let words_per_minute = if stats.total_recording_seconds > 0.0 {
        stats.total_words as f64 / (stats.total_recording_seconds / 60.0)
    } else {
        0.0
    };
    let average_transcription_length = if stats.total_transcriptions > 0 {
        stats.total_words as f64 / stats.total_transcriptions as f64
    } else {
        0.0
    };

    UsageStatsReport {
        total_transcriptions: stats.total_transcriptions,
        total_words: stats.total_words,
        total_recording_seconds: stats.total_recording_seconds,
        words_per_minute,
        average_transcription_length,
        streak_days: calculate_streak_days(&stats.daily_stats, today),
        today: today_stats,
        daily_word_history: build_daily_word_history(&stats.daily_stats, today, history_days),
        last_updated: stats.last_updated.clone(),
    }
}

fn calculate_streak_days(daily_stats: &BTreeMap<String, DailyStats>, today: NaiveDate) -> u64 {
    let mut streak = 0_u64;
    let mut cursor = today;

    loop {
        let cursor_key = date_key(cursor);
        let has_activity = daily_stats
            .get(&cursor_key)
            .map(|stats| stats.transcriptions > 0)
            .unwrap_or(false);

        if !has_activity {
            break;
        }

        streak = streak.saturating_add(1);
        let Some(previous_date) = cursor.checked_sub_signed(Duration::days(1)) else {
            break;
        };
        cursor = previous_date;
    }

    streak
}

fn build_daily_word_history(
    daily_stats: &BTreeMap<String, DailyStats>,
    today: NaiveDate,
    history_days: usize,
) -> Vec<DailyWordCount> {
    if history_days == 0 {
        return Vec::new();
    }

    (0..history_days)
        .map(|offset| {
            let days_ago = (history_days - 1 - offset) as i64;
            let date = today
                .checked_sub_signed(Duration::days(days_ago))
                .unwrap_or(today);
            let date_key = date_key(date);
            let words = daily_stats
                .get(&date_key)
                .map(|stats| stats.words)
                .unwrap_or(0);
            DailyWordCount {
                date: date_key,
                words,
            }
        })
        .collect()
}

fn sanitize_seconds(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

fn today_local_date() -> NaiveDate {
    Local::now().date_naive()
}

fn today_date_key() -> String {
    date_key(today_local_date())
}

fn date_key(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn parse_date_key(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").ok()
}

fn temp_file_path_for(file_path: &Path) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = file_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(STATS_FILE_NAME);

    file_path.with_file_name(format!(
        ".{file_name}.{}.{timestamp}.tmp",
        std::process::id()
    ))
}

fn backup_corrupt_stats_file(file_path: &Path) -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = file_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(STATS_FILE_NAME);
    let backup_path = file_path.with_file_name(format!(
        "{file_name}.corrupt-{}-{timestamp}.bak",
        std::process::id()
    ));

    fs::rename(file_path, &backup_path).map_err(|error| {
        format!(
            "Failed to backup malformed usage stats file `{}` to `{}`: {error}",
            file_path.display(),
            backup_path.display()
        )
    })?;

    Ok(backup_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_store() -> (StatsStore, PathBuf, PathBuf) {
        let test_dir = std::env::temp_dir().join(format!("voice-stats-store-{}", Uuid::new_v4()));
        let file_path = test_dir.join(STATS_FILE_NAME);
        let store = StatsStore::new_with_file_path(file_path.clone())
            .expect("stats store should initialize for tests");
        (store, file_path, test_dir)
    }

    fn cleanup_test_dir(test_dir: &Path) {
        let _ = fs::remove_dir_all(test_dir);
    }

    fn corrupt_backup_paths(file_path: &Path) -> Vec<PathBuf> {
        let Some(parent_dir) = file_path.parent() else {
            return Vec::new();
        };
        let Some(file_name) = file_path.file_name().and_then(|name| name.to_str()) else {
            return Vec::new();
        };

        let mut backups = Vec::new();
        if let Ok(entries) = fs::read_dir(parent_dir) {
            for entry in entries.flatten() {
                if let Some(candidate) = entry.file_name().to_str() {
                    if candidate.starts_with(&format!("{file_name}.corrupt-"))
                        && candidate.ends_with(".bak")
                    {
                        backups.push(entry.path());
                    }
                }
            }
        }

        backups
    }

    fn assert_almost_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn record_transcription_updates_totals_and_today_stats() {
        let (store, _file_path, test_dir) = create_test_store();

        store
            .record_transcription(12, 45.5)
            .expect("stats recording should succeed");
        let report = store
            .get_usage_stats()
            .expect("stats should load after recording");

        assert_eq!(report.total_transcriptions, 1);
        assert_eq!(report.total_words, 12);
        assert_almost_eq(report.total_recording_seconds, 45.5);
        assert_eq!(report.today.transcriptions, 1);
        assert_eq!(report.today.words, 12);
        assert_almost_eq(report.today.recording_seconds, 45.5);
        assert_eq!(report.daily_word_history.len(), DEFAULT_HISTORY_WINDOW_DAYS);

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn metrics_accumulate_across_multiple_transcriptions() {
        let (store, _file_path, test_dir) = create_test_store();

        store
            .record_transcription(120, 60.0)
            .expect("first record should succeed");
        store
            .record_transcription(60, 30.0)
            .expect("second record should succeed");

        let report = store
            .get_usage_stats()
            .expect("stats should load after multiple records");
        assert_eq!(report.total_transcriptions, 2);
        assert_eq!(report.total_words, 180);
        assert_almost_eq(report.total_recording_seconds, 90.0);
        assert_almost_eq(report.words_per_minute, 120.0);
        assert_almost_eq(report.average_transcription_length, 90.0);

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn reset_usage_stats_clears_all_counters() {
        let (store, _file_path, test_dir) = create_test_store();

        store
            .record_transcription(25, 15.0)
            .expect("stats recording should succeed");
        store
            .reset_usage_stats()
            .expect("stats reset should succeed");

        let report = store
            .get_usage_stats()
            .expect("stats should load after reset");
        assert_eq!(report.total_transcriptions, 0);
        assert_eq!(report.total_words, 0);
        assert_almost_eq(report.total_recording_seconds, 0.0);
        assert_almost_eq(report.words_per_minute, 0.0);
        assert_almost_eq(report.average_transcription_length, 0.0);
        assert_eq!(report.streak_days, 0);
        assert_eq!(report.today, DailyStats::default());
        assert!(report
            .daily_word_history
            .iter()
            .all(|point| point.words == 0));

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn streak_counts_consecutive_days_with_activity() {
        let (store, file_path, test_dir) = create_test_store();
        let today = today_local_date();
        let yesterday = today
            .checked_sub_signed(Duration::days(1))
            .expect("yesterday should be representable");
        let three_days_ago = today
            .checked_sub_signed(Duration::days(3))
            .expect("three days ago should be representable");

        let mut daily_stats = BTreeMap::new();
        daily_stats.insert(
            date_key(today),
            DailyStats {
                transcriptions: 2,
                words: 40,
                recording_seconds: 20.0,
            },
        );
        daily_stats.insert(
            date_key(yesterday),
            DailyStats {
                transcriptions: 1,
                words: 18,
                recording_seconds: 8.0,
            },
        );
        daily_stats.insert(
            date_key(three_days_ago),
            DailyStats {
                transcriptions: 1,
                words: 10,
                recording_seconds: 5.0,
            },
        );

        let seeded = UsageStats {
            total_transcriptions: 4,
            total_words: 68,
            total_recording_seconds: 33.0,
            daily_stats,
            last_updated: today_date_key(),
        };

        fs::write(
            &file_path,
            serde_json::to_vec_pretty(&seeded).expect("seeded stats should serialize"),
        )
        .expect("seeded usage stats file should be writable");

        let report = store
            .get_usage_stats()
            .expect("report should load seeded stats");
        assert_eq!(report.streak_days, 2);
        assert_eq!(report.today.words, 40);

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn recovers_from_malformed_stats_file() {
        let (store, file_path, test_dir) = create_test_store();
        fs::write(&file_path, "{ malformed json")
            .expect("test should be able to write malformed stats json");

        let report = store
            .get_usage_stats()
            .expect("store should recover malformed stats file");
        assert_eq!(report.total_transcriptions, 0);
        assert_eq!(corrupt_backup_paths(&file_path).len(), 1);

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn negative_or_non_finite_duration_is_safely_clamped() {
        let (store, _file_path, test_dir) = create_test_store();

        store
            .record_transcription(5, f64::NAN)
            .expect("stats record should ignore NaN duration");
        store
            .record_transcription(5, -10.0)
            .expect("stats record should clamp negative duration");

        let report = store.get_usage_stats().expect("stats should load");
        assert_eq!(report.total_transcriptions, 2);
        assert_eq!(report.total_words, 10);
        assert_almost_eq(report.total_recording_seconds, 0.0);

        cleanup_test_dir(&test_dir);
    }
}
