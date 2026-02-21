use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

const HISTORY_FILE_NAME: &str = "transcript_history.json";
pub const MAX_HISTORY_PAGE_SIZE: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub text: String,
    pub timestamp: String,
    #[serde(default)]
    pub duration_secs: Option<f64>,
    #[serde(default)]
    pub language: Option<String>,
    pub provider: String,
}

impl HistoryEntry {
    pub fn new(
        text: String,
        duration_secs: Option<f64>,
        language: Option<String>,
        provider: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            text,
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            duration_secs,
            language: normalize_optional(language),
            provider: provider.trim().to_string(),
        }
    }
}

#[derive(Debug)]
pub struct HistoryStore {
    file_path: PathBuf,
    io_lock: Mutex<()>,
}

impl HistoryStore {
    pub fn new(app: &AppHandle) -> Result<Self, String> {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("Failed to resolve app data directory: {error}"))?;

        Self::new_with_file_path(app_data_dir.join(HISTORY_FILE_NAME))
    }

    pub fn new_with_file_path(file_path: PathBuf) -> Result<Self, String> {
        ensure_history_file(&file_path)?;
        Ok(Self {
            file_path,
            io_lock: Mutex::new(()),
        })
    }

    pub fn add_entry(&self, entry: HistoryEntry) -> Result<(), String> {
        validate_entry(&entry)?;

        let _guard = self
            .io_lock
            .lock()
            .map_err(|_| "History store lock is poisoned".to_string())?;
        let mut entries = self.read_entries()?;

        let insert_at = entries.partition_point(|existing| existing.timestamp >= entry.timestamp);
        entries.insert(insert_at, entry);

        self.write_entries(&entries)
    }

    pub fn list_entries(&self, limit: usize, offset: usize) -> Result<Vec<HistoryEntry>, String> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let _guard = self
            .io_lock
            .lock()
            .map_err(|_| "History store lock is poisoned".to_string())?;
        let entries = self.read_entries()?;

        Ok(entries
            .into_iter()
            .skip(offset)
            .take(limit.min(MAX_HISTORY_PAGE_SIZE))
            .collect())
    }

    pub fn get_entry(&self, id: &str) -> Result<Option<HistoryEntry>, String> {
        let _guard = self
            .io_lock
            .lock()
            .map_err(|_| "History store lock is poisoned".to_string())?;
        let entries = self.read_entries()?;

        Ok(entries.into_iter().find(|entry| entry.id == id))
    }

    pub fn delete_entry(&self, id: &str) -> Result<bool, String> {
        let _guard = self
            .io_lock
            .lock()
            .map_err(|_| "History store lock is poisoned".to_string())?;
        let mut entries = self.read_entries()?;
        let original_len = entries.len();

        entries.retain(|entry| entry.id != id);
        let deleted = entries.len() != original_len;

        if deleted {
            self.write_entries(&entries)?;
        }

        Ok(deleted)
    }

    pub fn clear_history(&self) -> Result<(), String> {
        let _guard = self
            .io_lock
            .lock()
            .map_err(|_| "History store lock is poisoned".to_string())?;
        self.write_entries(&[])
    }

    fn read_entries(&self) -> Result<Vec<HistoryEntry>, String> {
        if !self.file_path.exists() {
            return Ok(Vec::new());
        }

        let raw_contents = fs::read_to_string(&self.file_path)
            .map_err(|error| format!("Failed to read transcript history file: {error}"))?;

        if raw_contents.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut entries = match serde_json::from_str::<Vec<HistoryEntry>>(&raw_contents) {
            Ok(parsed) => parsed,
            Err(error) => {
                self.recover_malformed_history_file(format!(
                    "Failed to parse transcript history file: {error}"
                ))?;
                return Ok(Vec::new());
            }
        };

        if let Err(error) = entries.iter().try_for_each(validate_entry) {
            self.recover_malformed_history_file(format!(
                "Failed to validate transcript history file: {error}"
            ))?;
            return Ok(Vec::new());
        }

        if !entries
            .windows(2)
            .all(|window| window[0].timestamp >= window[1].timestamp)
        {
            entries.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
        }

        Ok(entries)
    }

    fn write_entries(&self, entries: &[HistoryEntry]) -> Result<(), String> {
        let serialized = serde_json::to_vec_pretty(entries)
            .map_err(|error| format!("Failed to serialize transcript history entries: {error}"))?;
        let temp_path = temp_file_path_for(&self.file_path);

        let mut temp_file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)
            .map_err(|error| {
                format!(
                    "Failed to create transcript history temp file `{}`: {error}",
                    temp_path.display()
                )
            })?;

        if let Err(error) = temp_file.write_all(&serialized) {
            let _ = fs::remove_file(&temp_path);
            return Err(format!(
                "Failed to write transcript history temp file `{}`: {error}",
                temp_path.display()
            ));
        }

        if let Err(error) = temp_file.sync_all() {
            let _ = fs::remove_file(&temp_path);
            return Err(format!(
                "Failed to flush transcript history temp file `{}`: {error}",
                temp_path.display()
            ));
        }

        drop(temp_file);

        fs::rename(&temp_path, &self.file_path).map_err(|error| {
            let _ = fs::remove_file(&temp_path);
            format!("Failed to finalize transcript history file: {error}")
        })?;

        Ok(())
    }

    fn recover_malformed_history_file(&self, reason: String) -> Result<(), String> {
        let backup_path = backup_corrupt_history_file(&self.file_path)?;
        self.write_entries(&[])?;
        eprintln!(
            "Recovered malformed history file `{}` (backup: `{}`): {reason}",
            self.file_path.display(),
            backup_path.display(),
        );
        Ok(())
    }
}

fn ensure_history_file(file_path: &Path) -> Result<(), String> {
    if let Some(parent_dir) = file_path.parent() {
        fs::create_dir_all(parent_dir)
            .map_err(|error| format!("Failed to create history directory: {error}"))?;
    }

    if !file_path.exists() {
        fs::write(file_path, "[]")
            .map_err(|error| format!("Failed to initialize history file: {error}"))?;
    }

    Ok(())
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn temp_file_path_for(file_path: &Path) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = file_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("transcript_history.json");

    file_path.with_file_name(format!(
        ".{file_name}.{}.{timestamp}.tmp",
        std::process::id()
    ))
}

fn backup_corrupt_history_file(file_path: &Path) -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = file_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("transcript_history.json");
    let backup_path = file_path.with_file_name(format!(
        "{file_name}.corrupt-{}-{timestamp}.bak",
        std::process::id()
    ));

    fs::rename(file_path, &backup_path).map_err(|error| {
        format!(
            "Failed to backup malformed history file `{}` to `{}`: {error}",
            file_path.display(),
            backup_path.display()
        )
    })?;

    Ok(backup_path)
}

fn validate_entry(entry: &HistoryEntry) -> Result<(), String> {
    if entry.id.trim().is_empty() {
        return Err("History entry id cannot be empty".to_string());
    }

    if entry.text.trim().is_empty() {
        return Err("History entry text cannot be empty".to_string());
    }

    if entry.timestamp.trim().is_empty() {
        return Err("History entry timestamp cannot be empty".to_string());
    }

    if entry.provider.trim().is_empty() {
        return Err("History entry provider cannot be empty".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_store() -> (HistoryStore, PathBuf, PathBuf) {
        let test_dir = std::env::temp_dir().join(format!("voice-history-store-{}", Uuid::new_v4()));
        let file_path = test_dir.join(HISTORY_FILE_NAME);
        let store = HistoryStore::new_with_file_path(file_path.clone())
            .expect("history store should initialize for tests");

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

    fn test_entry(text: &str, timestamp: &str) -> HistoryEntry {
        HistoryEntry {
            id: Uuid::new_v4().to_string(),
            text: text.to_string(),
            timestamp: timestamp.to_string(),
            duration_secs: Some(2.5),
            language: Some("en".to_string()),
            provider: "openai".to_string(),
        }
    }

    #[test]
    fn supports_add_get_delete_and_clear() {
        let (store, _file_path, test_dir) = create_test_store();

        let entry = HistoryEntry::new(
            "first transcript".to_string(),
            Some(1.2),
            Some("en".to_string()),
            "openai".to_string(),
        );
        let entry_id = entry.id.clone();

        store
            .add_entry(entry.clone())
            .expect("entry should be added successfully");

        let listed = store
            .list_entries(10, 0)
            .expect("entries should list successfully");
        assert_eq!(listed, vec![entry.clone()]);

        let loaded = store
            .get_entry(&entry_id)
            .expect("entry lookup should succeed");
        assert_eq!(loaded, Some(entry));

        let deleted = store
            .delete_entry(&entry_id)
            .expect("entry deletion should succeed");
        assert!(deleted);

        assert!(store
            .get_entry(&entry_id)
            .expect("lookup should succeed after deletion")
            .is_none());

        store
            .add_entry(HistoryEntry::new(
                "second transcript".to_string(),
                None,
                None,
                "openai".to_string(),
            ))
            .expect("entry should be added successfully");
        store
            .clear_history()
            .expect("history should be cleared successfully");

        assert!(store
            .list_entries(10, 0)
            .expect("listing should succeed after clear")
            .is_empty());

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn lists_newest_first_with_pagination() {
        let (store, _file_path, test_dir) = create_test_store();

        let oldest = test_entry("oldest", "2026-01-01T09:00:00Z");
        let newest = test_entry("newest", "2026-01-01T11:00:00Z");
        let middle = test_entry("middle", "2026-01-01T10:00:00Z");

        store
            .add_entry(oldest.clone())
            .expect("oldest should be added");
        store
            .add_entry(newest.clone())
            .expect("newest should be added");
        store
            .add_entry(middle.clone())
            .expect("middle should be added");

        let page = store
            .list_entries(2, 1)
            .expect("paginated listing should succeed");

        assert_eq!(page, vec![middle, oldest]);
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn list_entries_handles_zero_limit_and_large_offset() {
        let (store, _file_path, test_dir) = create_test_store();

        store
            .add_entry(HistoryEntry::new(
                "sample".to_string(),
                Some(1.0),
                Some("en".to_string()),
                "openai".to_string(),
            ))
            .expect("entry should be added");

        assert!(store
            .list_entries(0, 0)
            .expect("zero-limit listing should succeed")
            .is_empty());
        assert!(store
            .list_entries(10, 99)
            .expect("large-offset listing should succeed")
            .is_empty());

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn delete_and_get_non_existent_entry_are_safe() {
        let (store, _file_path, test_dir) = create_test_store();
        let missing_id = Uuid::new_v4().to_string();

        assert!(!store
            .delete_entry(&missing_id)
            .expect("deleting a missing entry should succeed"));
        assert!(store
            .get_entry(&missing_id)
            .expect("lookup for missing entry should succeed")
            .is_none());

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn rejects_entries_with_missing_required_fields() {
        let (store, _file_path, test_dir) = create_test_store();
        let invalid_entry = HistoryEntry {
            id: String::new(),
            text: "hello".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            duration_secs: None,
            language: None,
            provider: "openai".to_string(),
        };

        let error = store
            .add_entry(invalid_entry)
            .expect_err("entry with an empty id should be rejected");
        assert!(error.contains("id"));

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn reports_invalid_json_file_contents() {
        let (store, file_path, test_dir) = create_test_store();

        fs::write(&file_path, "{ not valid json")
            .expect("test should be able to write malformed json");
        let listed = store
            .list_entries(10, 0)
            .expect("malformed json should be recovered automatically");

        assert!(listed.is_empty());
        assert_eq!(corrupt_backup_paths(&file_path).len(), 1);
        assert_eq!(
            fs::read_to_string(&file_path).expect("recovered history file should be readable"),
            "[]"
        );
        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn list_entries_enforces_max_page_size() {
        let (store, file_path, test_dir) = create_test_store();
        let entry_count = MAX_HISTORY_PAGE_SIZE + 5;
        let entries: Vec<HistoryEntry> = (0..entry_count)
            .map(|index| HistoryEntry {
                id: Uuid::new_v4().to_string(),
                text: format!("entry-{index}"),
                timestamp: format!("2026-01-01T00:{:02}:{:02}Z", (index / 60) % 60, index % 60),
                duration_secs: None,
                language: None,
                provider: "openai".to_string(),
            })
            .collect();
        fs::write(
            &file_path,
            serde_json::to_vec_pretty(&entries).expect("entries should serialize"),
        )
        .expect("history file should be written");

        let page = store
            .list_entries(usize::MAX, 0)
            .expect("list should respect page cap");

        assert_eq!(page.len(), MAX_HISTORY_PAGE_SIZE);
        cleanup_test_dir(&test_dir);
    }
}
