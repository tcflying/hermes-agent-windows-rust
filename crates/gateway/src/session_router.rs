use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionSource {
    pub platform: String,
    pub chat_id: String,
    pub chat_type: String,
    pub user_id: String,
    pub thread_id: Option<String>,
}

impl SessionSource {
    pub fn key(&self) -> String {
        let thread = self.thread_id.as_deref().unwrap_or("none");
        format!(
            "agent:main:{}:{}:{}:{}:{}",
            self.platform, self.chat_type, self.chat_id, thread, self.user_id
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SessionMapping {
    source_key: String,
    session_id: String,
    source: SessionSource,
    created_at: String,
}

fn get_store_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hermes")
        .join("session_routes.json")
}

fn load_mappings() -> HashMap<String, SessionMapping> {
    let path = get_store_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_mappings(mappings: &HashMap<String, SessionMapping>) -> Result<(), String> {
    let path = get_store_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = serde_json::to_string_pretty(mappings).map_err(|e| format!("{}", e))?;
    fs::write(&path, content).map_err(|e| format!("{}", e))
}

pub struct SessionRouter {
    mappings: HashMap<String, SessionMapping>,
}

impl SessionRouter {
    pub fn new() -> Self {
        Self {
            mappings: load_mappings(),
        }
    }

    pub fn resolve_key(source: &SessionSource) -> String {
        source.key()
    }

    pub fn resolve_session(&mut self, source: &SessionSource) -> Option<String> {
        let key = source.key();
        self.mappings.get(&key).map(|m| m.session_id.clone())
    }

    pub fn add_mapping(&mut self, source: SessionSource, session_id: String) -> Result<(), String> {
        let key = source.key();
        let mapping = SessionMapping {
            source_key: key.clone(),
            session_id,
            source,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.mappings.insert(key, mapping);
        save_mappings(&self.mappings)
    }

    pub fn get_session(&self, source: &SessionSource) -> Option<String> {
        let key = source.key();
        self.mappings.get(&key).map(|m| m.session_id.clone())
    }

    pub fn list_sessions(&self, platform: &str) -> String {
        let sessions: Vec<&SessionMapping> = self
            .mappings
            .values()
            .filter(|m| m.source.platform == platform)
            .collect();
        serde_json::json!({
            "platform": platform,
            "count": sessions.len(),
            "sessions": sessions
        })
        .to_string()
    }

    pub fn remove_session(&mut self, source: &SessionSource) -> bool {
        let key = source.key();
        let removed = self.mappings.remove(&key).is_some();
        if removed {
            let _ = save_mappings(&self.mappings);
        }
        removed
    }

    pub fn list_all(&self) -> String {
        let entries: Vec<&SessionMapping> = self.mappings.values().collect();
        serde_json::json!({
            "total": entries.len(),
            "mappings": entries
        })
        .to_string()
    }
}

impl Default for SessionRouter {
    fn default() -> Self {
        Self::new()
    }
}
