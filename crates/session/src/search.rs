use crate::db::{SessionDb, SessionSearchResult};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SessionSearch {
    db: Arc<Mutex<SessionDb>>,
}

impl SessionSearch {
    pub fn new(db: Arc<Mutex<SessionDb>>) -> Self {
        Self { db }
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SessionSearchResult>> {
        let db = self.db.lock().await;
        db.search_sessions(query, limit).await
    }
}
