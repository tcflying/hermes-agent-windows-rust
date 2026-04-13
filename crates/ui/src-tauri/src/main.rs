#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{Manager, Emitter};
use serde::{Deserialize, Serialize};

const API_BASE: &str = "http://localhost:3848";

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessageReq {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatRequestReq {
    model: String,
    messages: Vec<ChatMessageReq>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatResponseRes {
    content: String,
    #[serde(default)]
    session_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SessionInfoRes {
    id: String,
    created_at: String,
    updated_at: String,
    model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfigRes {
    model: String,
    provider: String,
    api_url: String,
    api_key: String,
    skin: String,
}

#[tauri::command]
async fn chat(request: ChatRequestReq) -> Result<ChatResponseRes, String> {
    let client = reqwest::Client::new();
    let resp = client.post(format!("{}/api/chat", API_BASE))
        .json(&request)
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }
    resp.json::<ChatResponseRes>().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn chat_stream(app: tauri::AppHandle, request: ChatRequestReq) -> Result<String, String> {
    let client = reqwest::Client::new();
    let mut resp = client.post(format!("{}/api/chat/stream", API_BASE))
        .json(&request)
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }

    let mut session_id: Option<String> = None;
    let mut buffer = String::new();

    while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
        if let Ok(text) = String::from_utf8(chunk.to_vec()) {
            buffer.push_str(&text);
        }
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].to_string();
            buffer = buffer[pos + 1..].to_string();
            if line.starts_with("data: ") {
                let data = &line[6..];
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(sid) = parsed.get("session_id").and_then(|v| v.as_str()) {
                        session_id = Some(sid.to_string());
                    }
                    if let Some(content) = parsed.get("content").and_then(|v| v.as_str()) {
                        let _ = app.emit("chat-chunk", serde_json::json!({
                            "content": content,
                            "done": false
                        }));
                    }
                    if let Some(done) = parsed.get("done").and_then(|v| v.as_bool()) {
                        if done {
                            let _ = app.emit("chat-chunk", serde_json::json!({
                                "content": "",
                                "done": true
                            }));
                        }
                    }
                }
            }
        }
    }

    Ok(session_id.unwrap_or_default())
}

#[tauri::command]
async fn list_sessions() -> Result<Vec<SessionInfoRes>, String> {
    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/api/sessions", API_BASE))
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }
    resp.json::<Vec<SessionInfoRes>>().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_session_cmd(model: Option<String>) -> Result<SessionInfoRes, String> {
    let client = reqwest::Client::new();
    let resp = client.post(format!("{}/api/sessions", API_BASE))
        .json(&serde_json::json!({ "model": model }))
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }
    resp.json::<SessionInfoRes>().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_session(id: String) -> Result<SessionInfoRes, String> {
    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/api/sessions/{}", API_BASE, id))
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }
    resp.json::<SessionInfoRes>().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_session_messages(id: String) -> Result<Vec<serde_json::Value>, String> {
    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/api/sessions/{}/messages", API_BASE, id))
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }
    resp.json::<Vec<serde_json::Value>>().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_session_cmd(id: String) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let resp = client.delete(format!("{}/api/sessions/{}", API_BASE, id))
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }
    resp.json::<serde_json::Value>().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn health_check() -> Result<String, String> {
    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/health", API_BASE))
        .send().await.map_err(|e| e.to_string())?;
    resp.text().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_config_cmd() -> Result<ConfigRes, String> {
    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/api/config", API_BASE))
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }
    resp.json::<ConfigRes>().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_config_cmd(update: serde_json::Value) -> Result<ConfigRes, String> {
    let client = reqwest::Client::new();
    let resp = client.put(format!("{}/api/config", API_BASE))
        .json(&update)
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }
    resp.json::<ConfigRes>().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_dir_cmd(path: String) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let resp = client.post(format!("{}/api/files/list", API_BASE))
        .json(&serde_json::json!({ "path": path }))
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }
    resp.json::<serde_json::Value>().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn read_file_cmd(path: String) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let resp = client.post(format!("{}/api/files/read", API_BASE))
        .json(&serde_json::json!({ "path": path }))
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }
    resp.json::<serde_json::Value>().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn write_file_cmd(path: String, content: String) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let resp = client.post(format!("{}/api/files/write", API_BASE))
        .json(&serde_json::json!({ "path": path, "content": content }))
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }
    resp.json::<serde_json::Value>().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn exec_terminal_cmd(command: String, cwd: Option<String>) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let resp = client.post(format!("{}/api/terminal", API_BASE))
        .json(&serde_json::json!({ "command": command, "cwd": cwd }))
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }
    resp.json::<serde_json::Value>().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_tools_cmd() -> Result<Vec<serde_json::Value>, String> {
    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/api/tools", API_BASE))
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }
    resp.json::<Vec<serde_json::Value>>().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn interrupt_chat_cmd() -> Result<(), String> {
    let client = reqwest::Client::new();
    let resp = client.post(format!("{}/api/chat/interrupt", API_BASE))
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body));
    }
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            window.set_title("Hermes Agent").ok();
            #[cfg(debug_assertions)]
            {
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            chat,
            chat_stream,
            list_sessions,
            create_session_cmd,
            get_session,
            get_session_messages,
            delete_session_cmd,
            health_check,
            get_config_cmd,
            update_config_cmd,
            list_dir_cmd,
            read_file_cmd,
            write_file_cmd,
            exec_terminal_cmd,
            list_tools_cmd,
            interrupt_chat_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}