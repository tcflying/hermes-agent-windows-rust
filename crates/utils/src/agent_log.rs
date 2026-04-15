use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Global log bridge sender — accessible from any thread.
/// Replaces thread_local! which breaks when tokio::spawn moves work to a different worker thread.
static LOG_SENDER: Mutex<Option<std::sync::mpsc::Sender<LogEntry>>> = Mutex::new(None);

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub model: Option<String>,
    pub session_id: Option<String>,
}

impl LogEntry {
    pub fn now(
        level: &str,
        target: &str,
        message: &str,
        model: Option<String>,
        session_id: Option<String>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        let timestamp = format!("{:02}:{:02}:{:02}", h, m, s);
        Self {
            timestamp,
            level: level.to_string(),
            target: target.to_string(),
            message: message.to_string(),
            model,
            session_id,
        }
    }
}

/// Install a log bridge sender. Called once per request before run_conversation.
pub fn init_log_sender(sender: std::sync::mpsc::Sender<LogEntry>) {
    if let Ok(mut s) = LOG_SENDER.lock() {
        *s = Some(sender);
    }
}

/// Drop the log bridge sender. Called once per request after run_conversation finishes.
pub fn drop_log_sender() {
    if let Ok(mut s) = LOG_SENDER.lock() {
        *s = None;
    }
}

/// Send a log entry through the bridge (no-op if no sender is installed).
pub fn send_log(entry: LogEntry) {
    if let Ok(s) = LOG_SENDER.lock() {
        if let Some(ref sender) = *s {
            let _ = sender.send(entry);
        }
    }
}
