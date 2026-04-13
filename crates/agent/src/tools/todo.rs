use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TodoTask {
    pub id: String,
    pub content: String,
    pub status: String,
    pub priority: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct TodoStore {
    tasks: Vec<TodoTask>,
    file_path: PathBuf,
}

impl TodoStore {
    fn get_file_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".hermes")
            .join("todos.json")
    }

    fn now() -> String {
        chrono::Utc::now().to_rfc3339()
    }

    pub fn new() -> Self {
        let file_path = Self::get_file_path();
        let tasks = if let Ok(content) = fs::read_to_string(&file_path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };
        Self { tasks, file_path }
    }

    fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.file_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err(format!("Failed to create directory: {}", e));
            }
        }
        let content = serde_json::to_string_pretty(&self.tasks)
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        fs::write(&self.file_path, content).map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(())
    }

    pub fn add(&mut self, content: &str, priority: &str) -> TodoTask {
        let now = Self::now();
        let task = TodoTask {
            id: Uuid::new_v4().to_string(),
            content: content.to_string(),
            status: "pending".to_string(),
            priority: priority.to_string(),
            created_at: now.clone(),
            updated_at: now,
        };
        self.tasks.push(task.clone());
        let _ = self.save();
        task
    }

    pub fn complete(&mut self, id: &str) -> Option<TodoTask> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.status = "completed".to_string();
            task.updated_at = Self::now();
            let result = task.clone();
            let _ = self.save();
            return Some(result);
        }
        None
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let len_before = self.tasks.len();
        self.tasks.retain(|t| t.id != id);
        let removed = self.tasks.len() < len_before;
        if removed {
            let _ = self.save();
        }
        removed
    }

    pub fn list(&self) -> Vec<TodoTask> {
        self.tasks.clone()
    }

    pub fn update(&mut self, id: &str, content: &str) -> Option<TodoTask> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.content = content.to_string();
            task.updated_at = Self::now();
            let result = task.clone();
            let _ = self.save();
            return Some(result);
        }
        None
    }
}

pub fn execute_todo(
    action: &str,
    id: Option<&str>,
    content: Option<&str>,
    priority: Option<&str>,
) -> String {
    let mut store = TodoStore::new();

    match action {
        "add" => {
            let content = match content {
                Some(c) => c,
                None => {
                    return serde_json::json!({"error": "content is required for add action"})
                        .to_string()
                }
            };
            let priority = priority.unwrap_or("medium");
            let task = store.add(content, priority);
            serde_json::json!({
                "status": "added",
                "task": task
            })
            .to_string()
        }
        "complete" => {
            let id = match id {
                Some(i) => i,
                None => {
                    return serde_json::json!({"error": "id is required for complete action"})
                        .to_string()
                }
            };
            match store.complete(id) {
                Some(task) => serde_json::json!({
                    "status": "completed",
                    "task": task
                })
                .to_string(),
                None => serde_json::json!({"error": format!("Task not found: {}", id)}).to_string(),
            }
        }
        "remove" => {
            let id = match id {
                Some(i) => i,
                None => {
                    return serde_json::json!({"error": "id is required for remove action"})
                        .to_string()
                }
            };
            if store.remove(id) {
                serde_json::json!({
                    "status": "removed",
                    "id": id
                })
                .to_string()
            } else {
                serde_json::json!({"error": format!("Task not found: {}", id)}).to_string()
            }
        }
        "list" => {
            let tasks = store.list();
            serde_json::json!({
                "status": "success",
                "count": tasks.len(),
                "tasks": tasks
            })
            .to_string()
        }
        "update" => {
            let id = match id {
                Some(i) => i,
                None => {
                    return serde_json::json!({"error": "id is required for update action"})
                        .to_string()
                }
            };
            let content = match content {
                Some(c) => c,
                None => {
                    return serde_json::json!({"error": "content is required for update action"})
                        .to_string()
                }
            };
            match store.update(id, content) {
                Some(task) => serde_json::json!({
                    "status": "updated",
                    "task": task
                })
                .to_string(),
                None => serde_json::json!({"error": format!("Task not found: {}", id)}).to_string(),
            }
        }
        _ => serde_json::json!({"error": format!("Unknown action: {}", action)}).to_string(),
    }
}
