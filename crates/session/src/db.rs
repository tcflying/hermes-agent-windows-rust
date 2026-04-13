use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SessionDb {
    conn: Arc<Mutex<Connection>>,
}

impl SessionDb {
    pub fn new(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS sessions (
                 id TEXT PRIMARY KEY,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL,
                 model TEXT,
                 messages TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS messages (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 session_id TEXT NOT NULL,
                 role TEXT NOT NULL,
                 content TEXT NOT NULL,
                 created_at TEXT NOT NULL
             );
             CREATE VIRTUAL TABLE IF NOT EXISTS sessions_fts USING fts5(
                 session_id UNINDEXED,
                 content,
                 tokenize='porter unicode61'
             );",
        )?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    pub async fn save_message(&self, session_id: &str, role: &str, content: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO messages (session_id, role, content, created_at) VALUES (?1, ?2, ?3, datetime('now'))",
            params![session_id, role, content],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO sessions_fts (rowid, session_id, content) VALUES ((SELECT MAX(rowid) FROM messages WHERE session_id = ?1), ?1, ?2)",
            params![session_id, content],
        )?;
        Ok(())
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<Session>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, created_at, updated_at, model FROM sessions WHERE id = ?1",
        )?;
        let session = stmt
            .query_row(params![session_id], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    updated_at: row.get(2)?,
                    model: row.get(3)?,
                })
            })
            .optional()?;
        Ok(session)
    }

    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, created_at, updated_at, model FROM sessions ORDER BY updated_at DESC",
        )?;
        let sessions = stmt.query_map([], |row| {
            Ok(SessionInfo {
                id: row.get(0)?,
                created_at: row.get(1)?,
                updated_at: row.get(2)?,
                model: row.get::<_, Option<String>>(3)?,
            })
        })?;
        let mut result = Vec::new();
        for session in sessions {
            result.push(session?);
        }
        Ok(result)
    }

    pub async fn create_session(&self, model: Option<String>) -> Result<SessionInfo> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO sessions (id, created_at, updated_at, model, messages) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, now, now, model, "[]"],
        )?;
        Ok(SessionInfo {
            id,
            created_at: now.clone(),
            updated_at: now,
            model,
        })
    }

    pub async fn get_messages(&self, session_id: &str) -> Result<Vec<SessionMessage>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT role, content FROM messages WHERE session_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(SessionMessage {
                role: row.get(0)?,
                content: row.get(1)?,
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM messages WHERE session_id = ?1", params![session_id])?;
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;
        conn.execute("DELETE FROM sessions_fts WHERE session_id = ?1", params![session_id])?;
        Ok(())
    }

    pub async fn search_sessions(&self, query: &str, limit: usize) -> Result<Vec<SessionSearchResult>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT s.id, s.created_at, s.updated_at, s.model, snippet(sessions_fts, 1, '<mark>', '</mark>', '...', 64) as snippet \
             FROM sessions_fts \
             JOIN sessions s ON s.id = sessions_fts.session_id \
             WHERE sessions_fts MATCH ?1 \
             ORDER BY rank \
             LIMIT ?2"
        )?;

        let results = stmt.query_map(params![query, limit as i64], |row| {
            Ok(SessionSearchResult {
                session_id: row.get(0)?,
                created_at: row.get(1)?,
                updated_at: row.get(2)?,
                model: row.get::<_, Option<String>>(3)?,
                snippet: row.get(4)?,
            })
        })?;

        let mut vec = Vec::new();
        for r in results {
            vec.push(r?);
        }
        Ok(vec)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSearchResult {
    pub session_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub model: Option<String>,
    pub snippet: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_db_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("test_hermes_session_{}_{}.db", name, uuid::Uuid::new_v4()));
        path
    }

    async fn cleanup(path: &PathBuf) {
        let _ = tokio::fs::remove_file(path).await;
        let wal = path.with_extension("db-wal");
        let shm = path.with_extension("db-shm");
        let _ = tokio::fs::remove_file(&wal).await;
        let _ = tokio::fs::remove_file(&shm).await;
    }

    #[tokio::test]
    async fn test_session_db_new() {
        let path = temp_db_path("new");
        let result = SessionDb::new(path.clone());
        assert!(result.is_ok());
        cleanup(&path).await;
    }

    #[tokio::test]
    async fn test_create_and_get_session() {
        let path = temp_db_path("create_get");
        let db = SessionDb::new(path.clone()).unwrap();
        let created = db.create_session(Some("gpt-4".into())).await.unwrap();
        let fetched = db.get_session(&created.id).await.unwrap();
        assert!(fetched.is_some());
        let session = fetched.unwrap();
        assert_eq!(session.id, created.id);
        assert_eq!(session.model, Some("gpt-4".into()));
        cleanup(&path).await;
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let path = temp_db_path("list");
        let db = SessionDb::new(path.clone()).unwrap();
        db.create_session(None).await.unwrap();
        db.create_session(None).await.unwrap();
        let sessions = db.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);
        cleanup(&path).await;
    }

    #[tokio::test]
    async fn test_save_and_get_messages() {
        let path = temp_db_path("messages");
        let db = SessionDb::new(path.clone()).unwrap();
        let session = db.create_session(None).await.unwrap();
        db.save_message(&session.id, "user", "hello").await.unwrap();
        db.save_message(&session.id, "assistant", "hi there").await.unwrap();
        let messages = db.get_messages(&session.id).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "hello");
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].content, "hi there");
        cleanup(&path).await;
    }

    #[tokio::test]
    async fn test_delete_session() {
        let path = temp_db_path("delete");
        let db = SessionDb::new(path.clone()).unwrap();
        let session = db.create_session(None).await.unwrap();
        db.delete_session(&session.id).await.unwrap();
        let fetched = db.get_session(&session.id).await.unwrap();
        assert!(fetched.is_none());
        cleanup(&path).await;
    }

    #[tokio::test]
    async fn test_get_nonexistent_session() {
        let path = temp_db_path("nonexistent");
        let db = SessionDb::new(path.clone()).unwrap();
        let fetched = db.get_session("nonexistent").await.unwrap();
        assert!(fetched.is_none());
        cleanup(&path).await;
    }
}
