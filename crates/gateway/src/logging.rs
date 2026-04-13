use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

pub struct LogBuffer {
    entries: Vec<LogEntry>,
    max_entries: usize,
    file_path: Option<PathBuf>,
}

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
const MAX_ROTATED: usize = 5;

impl LogBuffer {
    pub fn new(max_entries: usize) -> Self {
        let file_path =
            dirs::home_dir().map(|h| h.join(".hermes").join("logs").join("agent.jsonl"));
        if let Some(ref path) = file_path {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
        }
        Self {
            entries: Vec::new(),
            max_entries,
            file_path,
        }
    }

    pub fn push(&mut self, entry: LogEntry) {
        if self.entries.len() >= self.max_entries {
            self.entries.remove(0);
        }
        self.entries.push(entry);
    }

    pub fn query(
        &self,
        level: Option<&str>,
        target: Option<&str>,
        limit: Option<usize>,
        since: Option<&str>,
    ) -> Vec<&LogEntry> {
        let mut results: Vec<&LogEntry> = self.entries.iter().collect();
        if let Some(lvl) = level {
            results.retain(|e| e.level == lvl);
        }
        if let Some(tgt) = target {
            results.retain(|e| e.target == tgt);
        }
        if let Some(s) = since {
            results.retain(|e| e.timestamp.as_str() >= s);
        }
        if let Some(lim) = limit {
            let start = results.len().saturating_sub(lim);
            results = results[start..].to_vec();
        }
        results
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn write_to_file(&self) -> std::io::Result<()> {
        let path = match &self.file_path {
            Some(p) => p,
            None => return Ok(()),
        };
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        for entry in &self.entries {
            let line = serde_json::to_string(entry).unwrap_or_default();
            writeln!(f, "{}", line)?;
        }
        Ok(())
    }

    pub fn rotate_if_needed(&mut self) -> std::io::Result<()> {
        let path = match &self.file_path.clone() {
            Some(p) => p.clone(),
            None => return Ok(()),
        };
        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => return Ok(()),
        };
        if metadata.len() < MAX_FILE_SIZE {
            return Ok(());
        }
        for i in (1..MAX_ROTATED).rev() {
            let old = path.with_extension(format!("jsonl.{}", i));
            let new = path.with_extension(format!("jsonl.{}", i + 1));
            if old.exists() {
                let _ = fs::rename(&old, &new);
            }
        }
        let _ = fs::rename(&path, path.with_extension("jsonl.1"));
        Ok(())
    }
}

pub fn log_agent(
    buffer: &Mutex<LogBuffer>,
    level: &str,
    target: &str,
    message: &str,
    model: Option<String>,
    session_id: Option<String>,
) {
    let entry = LogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        level: level.to_string(),
        target: target.to_string(),
        message: message.to_string(),
        model,
        session_id,
        metadata: None,
    };
    if let Ok(mut buf) = buffer.lock() {
        buf.push(entry);
    }
    eprintln!("[{}] [{}] {}", level, target, message);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(level: &str, target: &str, message: &str) -> LogEntry {
        LogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            level: level.to_string(),
            target: target.to_string(),
            message: message.to_string(),
            model: None,
            session_id: None,
            metadata: None,
        }
    }

    #[test]
    fn test_log_buffer_push_and_len() {
        let mut buf = LogBuffer::new(100);
        assert_eq!(buf.len(), 0);
        buf.push(make_entry("info", "test", "a"));
        buf.push(make_entry("info", "test", "b"));
        buf.push(make_entry("info", "test", "c"));
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn test_log_buffer_max_entries_eviction() {
        let mut buf = LogBuffer::new(2);
        buf.push(make_entry("info", "test", "first"));
        buf.push(make_entry("info", "test", "second"));
        buf.push(make_entry("info", "test", "third"));
        assert_eq!(buf.len(), 2);
        assert_eq!(buf.entries[0].message, "second");
        assert_eq!(buf.entries[1].message, "third");
    }

    #[test]
    fn test_log_buffer_query_by_level() {
        let mut buf = LogBuffer::new(100);
        buf.push(make_entry("info", "mod", "info msg"));
        buf.push(make_entry("error", "mod", "error msg"));
        buf.push(make_entry("warn", "mod", "warn msg"));
        buf.push(make_entry("error", "mod", "another error"));
        let errors = buf.query(Some("error"), None, None, None);
        assert_eq!(errors.len(), 2);
        assert!(errors.iter().all(|e| e.level == "error"));
    }

    #[test]
    fn test_log_buffer_query_with_limit() {
        let mut buf = LogBuffer::new(100);
        for i in 0..10 {
            buf.push(make_entry("info", "test", &format!("msg {}", i)));
        }
        let limited = buf.query(None, None, Some(3), None);
        assert_eq!(limited.len(), 3);
        assert_eq!(limited[0].message, "msg 7");
        assert_eq!(limited[1].message, "msg 8");
        assert_eq!(limited[2].message, "msg 9");
    }

    #[test]
    fn test_log_buffer_query_by_target() {
        let mut buf = LogBuffer::new(100);
        buf.push(make_entry("info", "gateway", "g1"));
        buf.push(make_entry("info", "agent", "a1"));
        buf.push(make_entry("info", "gateway", "g2"));
        buf.push(make_entry("info", "agent", "a2"));
        let gw = buf.query(None, Some("gateway"), None, None);
        assert_eq!(gw.len(), 2);
        assert!(gw.iter().all(|e| e.target == "gateway"));
    }

    #[test]
    fn test_log_buffer_query_by_since() {
        let mut buf = LogBuffer::new(100);
        let old = LogEntry {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            level: "info".to_string(),
            target: "test".to_string(),
            message: "old".to_string(),
            model: None,
            session_id: None,
            metadata: None,
        };
        let recent = LogEntry {
            timestamp: "2025-06-15T12:00:00Z".to_string(),
            level: "info".to_string(),
            target: "test".to_string(),
            message: "recent".to_string(),
            model: None,
            session_id: None,
            metadata: None,
        };
        buf.push(old);
        buf.push(recent);
        let filtered = buf.query(None, None, None, Some("2025-01-01T00:00:00Z"));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].message, "recent");
    }

    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry {
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            level: "info".to_string(),
            target: "gateway".to_string(),
            message: "test message".to_string(),
            model: Some("claude-4".to_string()),
            session_id: Some("sess-123".to_string()),
            metadata: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"timestamp\""));
        assert!(json.contains("\"level\""));
        assert!(json.contains("\"target\""));
        assert!(json.contains("\"message\""));
        assert!(json.contains("\"model\""));
        assert!(json.contains("\"session_id\""));
        assert!(json.contains("test message"));
    }
}
