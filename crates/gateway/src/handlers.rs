use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Json, sse::{Event, Sse}, IntoResponse},
    routing::{delete, get, post},
    Router,
};
use anyhow::Result;
use futures_util::TryStreamExt;
use hermes_agent::chat::{self, Message};
use hermes_agent::tools::skill_manager::SkillManager;
use hermes_agent::MemoryStore;
use hermes_config::{ConfigLoader, ConfigUpdate};
use hermes_session::{SessionDb, SessionInfo};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tokio_stream::wrappers::UnboundedReceiverStream;
use std::path::PathBuf;
use std::fs;
use std::time::Instant;
use tower_http::cors::{CorsLayer, Any};
use crate::env_handlers::env_routes;
use crate::config_ext_handlers::config_ext_routes;
use crate::cron_handlers::cron_routes;
use crate::status_handlers::status_routes;
use crate::skills_toolset_handlers::create_skills_toolset_router;
use crate::logging::LogBuffer;
use hermes_config::providers;

const DEFAULT_API_URL: &str = "https://api.minimaxi.com/v1";

#[derive(Clone)]
pub struct AppState {
    pub session_db: Arc<RwLock<SessionDb>>,
    pub config: Arc<RwLock<ConfigLoader>>,
    pub interrupt_flag: Arc<AtomicBool>,
    pub log_buffer: Arc<Mutex<LogBuffer>>,
    pub start_time: Arc<Instant>,
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub api_url: Option<String>,
    pub api_key: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub content: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health))
        .route("/api/chat", post(chat))
        .route("/api/chat/stream", post(chat_stream))
        .route("/api/sessions", get(list_sessions).post(create_session))
        .route("/api/sessions/{id}", get(get_session).delete(delete_session))
        .route("/api/sessions/{id}/messages", get(get_session_messages))
        .route("/api/tools", get(list_tools))
        .route("/api/files/list", post(list_dir))
        .route("/api/files/read", post(read_file))
        .route("/api/files/write", post(write_file))
        .route("/api/terminal", post(exec_terminal))
        .route("/api/chat/interrupt", post(chat_interrupt))
        .route("/api/config", get(get_config).put(update_config))
        .route("/api/config/providers", get(list_providers))
        .route("/api/config/provider", post(set_provider))
        .route("/api/memory/read", get(memory_read))
        .route("/api/memory/action", post(memory_action))
        .route("/api/hud/stats", get(hud_stats))
        .route("/api/hud/growth", get(hud_growth))
        .route("/api/hud/health", get(hud_health))
        .route("/api/skills", get(list_skills))
        .route("/api/skills/create", post(create_skill))
        .route("/api/skills/{name}", delete(delete_skill))
        .route("/api/skills/growth", get(skills_growth))
        .route("/api/logs", get(get_logs))
        .route("/api/models/list", get(list_models))
        .route("/api/models/switch", post(switch_model))
        .merge(env_routes())
        .merge(config_ext_routes())
        .merge(cron_routes())
        .merge(status_routes())
        .merge(create_skills_toolset_router())
        .layer(cors)
        .with_state(state)
}

pub async fn start_server(port: u16) -> Result<()> {
    print!("\x1b[33m╔══════════════════════════════════════╗\x1b[0m\n");
    print!("\x1b[33m║     Hermes Agent Gateway v0.6.0      ║\x1b[0m\n");
    print!("\x1b[33m╚══════════════════════════════════════╝\x1b[0m\n");
    print!("\x1b[36m  Listening on http://0.0.0.0:{}\x1b[0m\n", port);
    print!("\x1b[36m  Endpoints: /health /api/chat /api/chat/stream\x1b[0m\n");
    print!("\x1b[36m  Sessions:  /api/sessions /api/sessions/{{id}}\x1b[0m\n");
    print!("\x1b[36m  Tools:     /api/terminal /api/files/* /api/tools\x1b[0m\n");
    print!("\x1b[36m  Memory:    /api/memory/read /api/memory/action\x1b[0m\n");
    print!("\n");

    let db_path = std::env::var("HERMES_DB")
        .map(|p| std::path::PathBuf::from(p))
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".hermes")
                .join("sessions.db")
        });

    let session_db = SessionDb::new(db_path)
        .map_err(|e| anyhow::anyhow!("Failed to open session DB: {}", e))?;
    print!("\x1b[32m  ✓ Session DB loaded\x1b[0m\n");

    let config_path = std::env::var("HERMES_CONFIG")
        .map(|p| std::path::PathBuf::from(p))
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".hermes")
                .join("config.yaml")
        });

    let mut config_loader = ConfigLoader::new();
    if let Err(e) = config_loader.load(config_path) {
        print!("\x1b[33m  ⚠ Config not loaded: {}\x1b[0m\n", e);
    } else {
        print!("\x1b[32m  ✓ Config loaded\x1b[0m\n");
    }

    let has_key = std::env::var("MINIMAX_API_KEY").map(|k| !k.is_empty()).unwrap_or(false);
    if has_key {
        print!("\x1b[32m  ✓ API key set\x1b[0m\n");
    } else {
        print!("\x1b[31m  ✗ No MINIMAX_API_KEY env var!\x1b[0m\n");
    }
    print!("\n");

    let state = AppState {
        session_db: Arc::new(RwLock::new(session_db)),
        config: Arc::new(RwLock::new(config_loader)),
        interrupt_flag: Arc::new(AtomicBool::new(false)),
        log_buffer: Arc::new(Mutex::new(LogBuffer::new(2000))),
        start_time: Arc::new(Instant::now()),
    };

    let app = create_router(state);

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let dist_dir = exe_dir.join("dist");
    let static_service = tower_http::services::ServeDir::new(&dist_dir)
        .fallback(tower_http::services::ServeFile::new(dist_dir.join("index.html")));
    let app = app
        .fallback_service(static_service);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> &'static str {
    "OK"
}

fn setup_log_bridge(log_buffer: &Arc<Mutex<LogBuffer>>) -> std::sync::mpsc::Receiver<hermes_utils::LogEntry> {
    let (tx, rx) = std::sync::mpsc::channel::<hermes_utils::LogEntry>();
    hermes_utils::init_log_sender(tx);
    rx
}

fn flush_log_bridge(rx: &std::sync::mpsc::Receiver<hermes_utils::LogEntry>, log_buffer: &Arc<Mutex<LogBuffer>>) {
    while let Ok(entry) = rx.try_recv() {
        if let Ok(mut buf) = log_buffer.lock() {
            buf.push(crate::logging::LogEntry {
                timestamp: entry.timestamp,
                level: entry.level,
                target: entry.target,
                message: entry.message,
                model: entry.model,
                session_id: entry.session_id,
                metadata: None,
            });
        }
    }
    hermes_utils::drop_log_sender();
}

async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, Json<ErrorResponse>)> {
    let config = state.config.read().await.get().clone();
    let api_url = req.api_url
        .or_else(|| std::env::var("MINIMAX_API_URL").ok())
        .unwrap_or_else(|| config.api_url.clone());
    let api_key = req.api_key
        .or_else(|| std::env::var("MINIMAX_API_KEY").ok())
        .unwrap_or_else(|| config.api_key.clone());
    let model = req.model.clone();

    let session_id = if let Some(ref sid) = req.session_id {
        sid.clone()
    } else {
        let new_session = state.session_db.write().await.create_session(Some(model.clone())).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;
        new_session.id
    };
    let sid_short = &session_id[..8.min(session_id.len())];
    let user_msg = req.messages.last().map(|m| m.content.clone()).unwrap_or_default();
    let preview = if user_msg.len() > 80 { format!("{}...", &user_msg[..user_msg.ceil_char_boundary(80)]) } else { user_msg };
    print!("\x1b[35m[chat] POST /api/chat session={} msgs={} user=\"{}\"\x1b[0m\n", sid_short, req.messages.len(), preview);

    for msg in &req.messages {
        let role = if msg.role == "user" { "user" } else if msg.role == "assistant" { "assistant" } else { &msg.role };
        state.session_db.write().await.save_message(&session_id, role, &msg.content)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;
    }

    let messages: Vec<Message> = req
        .messages
        .into_iter()
        .map(|m| Message {
            role: m.role,
            content: Some(m.content),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        })
        .collect();

    let log_rx = setup_log_bridge(&state.log_buffer);
    match chat::run_conversation(&model, &api_url, &api_key, messages, Some(state.interrupt_flag.clone()), None).await {
        Ok(response) => {
            flush_log_bridge(&log_rx, &state.log_buffer);
            state.session_db.write().await.save_message(&session_id, "assistant", &response.content)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;
            Ok(Json(ChatResponse {
                content: response.content,
                session_id: Some(session_id),
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
        }))),
    }
}

async fn chat_stream(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let config = state.config.read().await.get().clone();
    let api_url = req.api_url
        .or_else(|| std::env::var("MINIMAX_API_URL").ok())
        .unwrap_or_else(|| config.api_url.clone());
    let api_key = req.api_key
        .or_else(|| std::env::var("MINIMAX_API_KEY").ok())
        .unwrap_or_else(|| config.api_key.clone());
    let model = req.model.clone();

    let session_id = match req.session_id {
        Some(ref sid) => sid.clone(),
        None => {
            let new_session = state.session_db.write().await.create_session(Some(model.clone()))
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;
            new_session.id
        }
    };

    let sid_short = &session_id[..8.min(session_id.len())];
    let user_msg = req.messages.last().map(|m| m.content.clone()).unwrap_or_default();
    let preview = if user_msg.len() > 80 { format!("{}...", &user_msg[..user_msg.ceil_char_boundary(80)]) } else { user_msg };
    print!("\x1b[35m[stream] POST /api/chat/stream session={} msgs={} user=\"{}\"\x1b[0m\n", sid_short, req.messages.len(), preview);

    for msg in &req.messages {
        let role = if msg.role == "user" { "user" } else if msg.role == "assistant" { "assistant" } else { &msg.role };
        state.session_db.write().await.save_message(&session_id, role, &msg.content)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;
    }

    let messages: Vec<Message> = req
        .messages
        .into_iter()
        .map(|m| Message {
            role: m.role,
            content: Some(m.content),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        })
        .collect();

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    state.interrupt_flag.store(false, Ordering::SeqCst);
    let interrupt_flag = state.interrupt_flag.clone();

    let session_id_clone = session_id.clone();
    let state_clone = state.clone();
    let sid_log = session_id[..8.min(session_id.len())].to_string();

    let (token_tx, mut token_rx) = tokio::sync::mpsc::channel::<String>(100);

    let tx_clone = tx.clone();
    let token_tx_clone = token_tx.clone();
    let log_rx = setup_log_bridge(&state.log_buffer);
    let state_log = state.clone();
    tokio::spawn(async move {
        let result = chat::run_conversation(&model, &api_url, &api_key, messages, Some(interrupt_flag), Some(token_tx)).await;

        flush_log_bridge(&log_rx, &state_log.log_buffer);

        match result {
            Ok(response) => {
                let full_content = response.content;
                let total_chars_count = full_content.chars().count();
                let resp_preview = if full_content.len() > 100 { format!("{}...", &full_content[..full_content.ceil_char_boundary(100)]) } else { full_content.clone() };
                print!("\x1b[32m[stream] session={} total {} chars\x1b[0m\n", sid_log, total_chars_count);

                tx_clone.send(Ok(Event::default().data(format!(r#"{{"session_id":"{}"}}"#, session_id_clone)))).ok();

                if !full_content.is_empty() {
                    state_clone.session_db.write().await
                        .save_message(&session_id_clone, "assistant", &full_content)
                        .await
                        .ok();
                }

                tx_clone.send(Ok(Event::default().data(r#"{"done":true}"#))).ok();
                print!("\x1b[32m[stream] session={} ✓ complete ({} chars)\x1b[0m\n", sid_log, total_chars_count);
            }
            Err(e) => {
                print!("\x1b[31m[stream] session={} ✗ ERROR: {}\x1b[0m\n", sid_log, e);
                let error_data = serde_json::json!({
                    "error": e.to_string()
                });
                if let Ok(event) = serde_json::to_string(&error_data) {
                    tx_clone.send(Ok(Event::default().data(event))).ok();
                }
                tx_clone.send(Ok(Event::default().data(r#"{"done":true}"#))).ok();
            }
        }
    });

    tokio::spawn(async move {
        while let Some(token) = token_rx.recv().await {
            let event_data = serde_json::json!({
                "content": token,
                "done": false
            });
            if let Ok(event) = serde_json::to_string(&event_data) {
                tx.send(Ok(Event::default().data(event))).ok();
            }
        }
    });

    let stream = UnboundedReceiverStream::new(rx).map_err(|e: anyhow::Error| anyhow::anyhow!("Channel error: {}", e));
    Ok(Sse::new(stream))
}

async fn chat_interrupt(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    state.interrupt_flag.store(true, Ordering::SeqCst);
    print!("\x1b[33m[interrupt] Conversation interrupt requested\x1b[0m\n");
    Json(serde_json::json!({"status": "interrupted"}))
}

async fn list_sessions(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.session_db.read().await;
    let sessions = db.list_sessions().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))
    })?;
    let mut result = Vec::with_capacity(sessions.len());
    for s in &sessions {
        let message_count = db.count_messages(&s.id).await.unwrap_or(0);
        let started_at = chrono::DateTime::parse_from_rfc3339(&s.created_at)
            .map(|dt| dt.timestamp() as u64)
            .unwrap_or(0);
        let last_active = chrono::DateTime::parse_from_rfc3339(&s.updated_at)
            .map(|dt| dt.timestamp() as u64)
            .unwrap_or(0);
        result.push(serde_json::json!({
            "id": s.id,
            "source": serde_json::Value::Null,
            "model": s.model,
            "title": serde_json::Value::Null,
            "started_at": started_at,
            "ended_at": serde_json::Value::Null,
            "last_active": last_active,
            "is_active": false,
            "message_count": message_count,
            "tool_call_count": 0,
            "input_tokens": 0,
            "output_tokens": 0,
            "preview": serde_json::Value::Null,
        }));
    }
    Ok(Json(result))
}

async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<SessionInfo>, (StatusCode, Json<ErrorResponse>)> {
    match state.session_db.write().await.create_session(req.model).await {
        Ok(session) => Ok(Json(session)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
        }))),
    }
}

async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SessionInfo>, (StatusCode, Json<ErrorResponse>)> {
    match state.session_db.read().await.get_session(&id).await {
        Ok(Some(s)) => Ok(Json(SessionInfo {
            id: s.id,
            created_at: s.created_at,
            updated_at: s.updated_at,
            model: s.model,
        })),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(ErrorResponse {
            error: "Session not found".to_string(),
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
        }))),
    }
}

async fn get_session_messages(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.session_db.read().await.get_messages(&id).await {
        Ok(messages) => {
            let messages_json: Vec<serde_json::Value> = messages.iter().map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                    "timestamp": m.timestamp,
                })
            }).collect();
            Ok(Json(serde_json::json!({
                "session_id": id,
                "messages": messages_json,
            })))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
        }))),
    }
}

async fn list_tools() -> Json<Vec<serde_json::Value>> {
    Json(hermes_agent::get_tool_definitions())
}

async fn delete_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<()>, (StatusCode, Json<ErrorResponse>)> {
    match state.session_db.write().await.delete_session(&id).await {
        Ok(_) => Ok(Json(())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
        }))),
    }
}

#[derive(Debug, Deserialize)]
struct ListDirRequest {
    path: String,
}

#[derive(Debug, Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
}

async fn list_dir(
    Json(req): Json<ListDirRequest>,
) -> Result<Json<Vec<FileEntry>>, (StatusCode, Json<ErrorResponse>)> {
    let path = PathBuf::from(&req.path);
    
    match fs::read_dir(&path) {
        Ok(entries) => {
            let mut files = Vec::new();
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    files.push(FileEntry {
                        name: entry.file_name().to_string_lossy().to_string(),
                        path: entry.path().to_string_lossy().to_string(),
                        is_dir: metadata.is_dir(),
                        size: metadata.len(),
                    });
                }
            }
            files.sort_by(|a, b| {
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                }
            });
            Ok(Json(files))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Failed to read directory: {}", e),
        }))),
    }
}

#[derive(Debug, Deserialize)]
struct ReadFileRequest {
    path: String,
}

#[derive(Debug, Serialize)]
struct ReadFileResponse {
    content: String,
    encoding: String,
}

async fn read_file(
    Json(req): Json<ReadFileRequest>,
) -> Result<Json<ReadFileResponse>, (StatusCode, Json<ErrorResponse>)> {
    let path = PathBuf::from(&req.path);
    
    match fs::read_to_string(&path) {
        Ok(content) => Ok(Json(ReadFileResponse {
            content,
            encoding: "utf-8".to_string(),
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Failed to read file: {}", e),
        }))),
    }
}

#[derive(Debug, Deserialize)]
struct WriteFileRequest {
    path: String,
    content: String,
}

async fn write_file(
    Json(req): Json<WriteFileRequest>,
) -> Result<Json<()>, (StatusCode, Json<ErrorResponse>)> {
    let path = PathBuf::from(&req.path);
    
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                    error: format!("Failed to create directory: {}", e),
                })));
            }
        }
    }
    
    match fs::write(&path, &req.content) {
        Ok(_) => Ok(Json(())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Failed to write file: {}", e),
        }))),
    }
}

#[derive(Debug, Deserialize)]
struct TerminalRequest {
    command: String,
    cwd: Option<String>,
}

#[derive(Debug, Serialize)]
struct TerminalResponse {
    output: String,
    exit_code: i32,
}

async fn exec_terminal(
    Json(req): Json<TerminalRequest>,
) -> Result<Json<TerminalResponse>, (StatusCode, Json<ErrorResponse>)> {
    let cwd = req.cwd.as_ref().map(PathBuf::from);
    
    let output = if cfg!(windows) {
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(["/C", &req.command]);
        if let Some(ref dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.output()
    } else {
        let mut cmd = std::process::Command::new("sh");
        cmd.args(["-c", &req.command]);
        if let Some(ref dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.output()
    };
    
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let exit_code = out.status.code().unwrap_or(-1);
            Ok(Json(TerminalResponse {
                output: if stderr.is_empty() { stdout } else { format!("{}\n{}", stdout, stderr) },
                exit_code,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Failed to execute command: {}", e),
        }))),
    }
}

async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let config = state.config.read().await.get().clone();
    Ok(Json(serde_json::json!({
        "model": config.model,
        "provider": config.provider,
        "api_url": config.api_url,
        "api_key": config.api_key,
        "skin": config.display.skin,
        "platforms": {
            "telegram": {
                "bot_token": config.platforms.telegram.bot_token,
                "enabled": config.platforms.telegram.enabled,
            },
            "discord": {
                "bot_token": config.platforms.discord.bot_token,
                "enabled": config.platforms.discord.enabled,
            },
            "slack": {
                "bot_token": config.platforms.slack.bot_token,
                "enabled": config.platforms.slack.enabled,
            },
            "whatsapp": {
                "bridge_url": config.platforms.whatsapp.bridge_url,
                "api_token": config.platforms.whatsapp.api_token,
                "enabled": config.platforms.whatsapp.enabled,
            },
            "signal": {
                "http_url": config.platforms.signal.http_url,
                "account": config.platforms.signal.account,
                "enabled": config.platforms.signal.enabled,
            },
        }
    })))
}

async fn update_config(
    State(state): State<AppState>,
    Json(update): Json<ConfigUpdate>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if let Err(e) = state.config.write().await.update(update) {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
        })));
    }
    let config = state.config.read().await.get().clone();
    Ok(Json(serde_json::json!({
        "model": config.model,
        "provider": config.provider,
        "api_url": config.api_url,
        "api_key": config.api_key,
        "skin": config.display.skin,
        "platforms": {
            "telegram": { "bot_token": config.platforms.telegram.bot_token, "enabled": config.platforms.telegram.enabled },
            "discord": { "bot_token": config.platforms.discord.bot_token, "enabled": config.platforms.discord.enabled },
            "slack": { "bot_token": config.platforms.slack.bot_token, "enabled": config.platforms.slack.enabled },
            "whatsapp": { "bridge_url": config.platforms.whatsapp.bridge_url, "api_token": config.platforms.whatsapp.api_token, "enabled": config.platforms.whatsapp.enabled },
            "signal": { "http_url": config.platforms.signal.http_url, "account": config.platforms.signal.account, "enabled": config.platforms.signal.enabled },
        }
    })))
}

async fn list_providers() -> Json<serde_json::Value> {
    let catalog = providers::all_providers();
    let credentials = providers::detect_credentials();
    Json(serde_json::json!({
        "providers": catalog,
        "credentials": credentials.iter()
            .map(|(id, has_key)| (id.clone(), *has_key))
            .collect::<std::collections::HashMap<String, bool>>(),
    }))
}

#[derive(Debug, Deserialize)]
struct SetProviderRequest {
    provider_id: String,
    api_key: Option<String>,
    model: Option<String>,
}

async fn set_provider(
    State(state): State<AppState>,
    Json(req): Json<SetProviderRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let provider = providers::all_providers()
        .into_iter()
        .find(|p| p.id == req.provider_id)
        .ok_or_else(|| (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: format!("Unknown provider: {}", req.provider_id) }),
        ))?;

    let model = req.model.unwrap_or_else(|| {
        provider.models.first()
            .map(|m| m.id.clone())
            .unwrap_or_default()
    });

    let mut update = ConfigUpdate {
        model: Some(model),
        provider: Some(provider.id.clone()),
        api_url: Some(provider.base_url.clone()),
        api_key: req.api_key,
        skin: None,
        telegram_token: None,
        telegram_enabled: None,
        discord_token: None,
        discord_enabled: None,
        slack_token: None,
        slack_enabled: None,
        whatsapp_bridge_url: None,
        whatsapp_api_token: None,
        whatsapp_enabled: None,
        signal_http_url: None,
        signal_account: None,
        signal_enabled: None,
    };

    if let Some(ref key) = update.api_key {
        if !key.is_empty() {
            std::env::set_var(&provider.api_key_env, key);
        }
    }

    if let Err(e) = state.config.write().await.update(update) {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
        })));
    }

    let config = state.config.read().await.get().clone();
    Ok(Json(serde_json::json!({
        "model": config.model,
        "provider": config.provider,
        "api_url": config.api_url,
        "api_key": config.api_key,
    })))
}

#[derive(Debug, Deserialize)]
struct CreateSessionRequest {
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MemoryActionRequest {
    action: String,
    target: String,
    content: Option<String>,
    old_content: Option<String>,
}

async fn memory_read() -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut store = hermes_agent::MemoryStore::new();
    if let Err(e) = store.load_from_disk() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Failed to load memory: {}", e),
        })));
    }
    let memory_content = store.execute_action("read", "memory", None, None);
    let user_content = store.execute_action("read", "user", None, None);
    Ok(Json(serde_json::json!({
        "memory": serde_json::from_str::<serde_json::Value>(&memory_content).unwrap_or_default(),
        "user": serde_json::from_str::<serde_json::Value>(&user_content).unwrap_or_default(),
    })))
}

async fn memory_action(
    Json(req): Json<MemoryActionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut store = hermes_agent::MemoryStore::new();
    if let Err(e) = store.load_from_disk() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Failed to load memory: {}", e),
        })));
    }
    let result = store.execute_action(&req.action, &req.target, req.content.as_deref(), req.old_content.as_deref());
    match serde_json::from_str::<serde_json::Value>(&result) {
        Ok(v) => Ok(Json(v)),
        Err(_) => Ok(Json(serde_json::json!({"raw": result}))),
    }
}

async fn hud_stats(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let config = state.config.read().await.get().clone();
    let uptime = state.start_time.elapsed().as_secs();
    let session_count = state.session_db.read().await.list_sessions().await
        .map(|s| s.len()).unwrap_or(0);
    let mut msg_count = 0;
    if let Ok(sessions) = state.session_db.read().await.list_sessions().await {
        for s in &sessions {
            if let Ok(msgs) = state.session_db.read().await.get_messages(&s.id).await {
                msg_count += msgs.len();
            }
        }
    }
    let skill_mgr = SkillManager::new();
    let skills = serde_json::from_str::<serde_json::Value>(&skill_mgr.list())
        .ok()
        .and_then(|v| v.as_array().map(|a| a.len()))
        .unwrap_or(0);
    Json(serde_json::json!({
        "total_sessions": session_count,
        "total_messages": msg_count,
        "total_skills": skills,
        "active_model": config.model,
        "uptime_seconds": uptime,
        "backend_status": "online",
    }))
}

async fn hud_growth() -> Json<serde_json::Value> {
    let skill_mgr = SkillManager::new();
    let list_str = skill_mgr.list();
    let skills: Vec<serde_json::Value> = serde_json::from_str(&list_str).unwrap_or_default();
    let mut by_date: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
    for skill in &skills {
        let date = skill.get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let name = skill.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        by_date.entry(date).or_default().push(name);
    }
    let mut result = Vec::new();
    let mut running = 0;
    for (date, names) in by_date {
        running += names.len();
        result.push(serde_json::json!({
            "date": date,
            "count": running,
            "new_skills": names,
        }));
    }
    Json(serde_json::json!({"skills_over_time": result}))
}

async fn hud_health(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let config = state.config.read().await.get().clone();
    let db_ok = state.session_db.read().await.list_sessions().await.is_ok();
    let mut mem_store = hermes_agent::MemoryStore::new();
    let mem_entries = if mem_store.load_from_disk().is_ok() {
        serde_json::from_str::<serde_json::Value>(&mem_store.execute_action("read", "memory", None, None))
            .ok()
            .and_then(|v| v.as_array().map(|a| a.len()))
            .unwrap_or(0)
    } else { 0 };
    let skill_mgr = SkillManager::new();
    let skill_count = serde_json::from_str::<serde_json::Value>(&skill_mgr.list())
        .ok()
        .and_then(|v| v.as_array().map(|a| a.len()))
        .unwrap_or(0);
    Json(serde_json::json!({
        "api_reachable": true,
        "model": config.model,
        "provider": config.provider,
        "last_error": serde_json::Value::Null,
        "sessions_db_ok": db_ok,
        "memory_entries": mem_entries,
        "skills_count": skill_count,
    }))
}

#[derive(Debug, Deserialize)]
struct LogsQuery {
    file: Option<String>,
    lines: Option<usize>,
    level: Option<String>,
    component: Option<String>,
}

async fn get_logs(
    State(state): State<AppState>,
    Query(q): Query<LogsQuery>,
) -> Json<serde_json::Value> {
    let file_name = q.file.as_deref().unwrap_or("agent").to_string();
    let line_count = q.lines.unwrap_or(100);

    // Map frontend level to internal level filter.
    // Frontend sends: ALL, DEBUG, INFO, WARNING, ERROR
    // Internal levels: debug, info, warn, error
    let level_filter: Option<String> = match q.level.as_deref() {
        Some("ALL") | None => None,
        Some("WARNING") => Some("warn".to_string()),
        Some(lvl) => Some(lvl.to_lowercase()),
    };
    let level_filter = level_filter.as_deref();

    // Map frontend component to internal target filter.
    // Frontend sends: all, gateway, agent, tools, cli, cron
    let target_filter: Option<&str> = match q.component.as_deref() {
        Some("all") | None => None,
        Some(c) => Some(c),
    };

    let buf = state.log_buffer.lock().unwrap();
    let entries = buf.query(level_filter, target_filter, Some(line_count), None);
    let lines: Vec<String> = entries.iter()
        .map(|e| format!("[{}] {}", e.level.to_uppercase(), e.message))
        .collect();
    Json(serde_json::json!({
        "file": file_name,
        "lines": lines,
    }))
}

async fn list_models(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let config = state.config.read().await.get().clone();
    let catalog = providers::all_providers();
    let mut models = Vec::new();
    for p in &catalog {
        for m in &p.models {
            models.push(serde_json::json!({
                "id": m.id,
                "provider": p.id,
                "name": m.name,
            }));
        }
    }
    Json(serde_json::json!({
        "models": models,
        "current": config.model,
    }))
}

#[derive(Debug, Deserialize)]
struct SwitchModelRequest {
    model: String,
}

async fn switch_model(
    State(state): State<AppState>,
    Json(req): Json<SwitchModelRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let resolved = providers::resolve_model(&req.model);
    let (api_url, api_key_env, model_id) = match resolved {
        Some(r) => r,
        None => (state.config.read().await.get().api_url.clone(), "MINIMAX_API_KEY".to_string(), req.model.clone()),
    };
    let api_key = std::env::var(&api_key_env)
        .or_else(|_| std::env::var("MINIMAX_API_KEY".to_string()))
        .unwrap_or_default();
    let update = ConfigUpdate {
        model: Some(model_id.clone()),
        provider: None,
        api_url: Some(api_url),
        api_key: if api_key.is_empty() { None } else { Some(api_key) },
        skin: None,
        telegram_token: None,
        telegram_enabled: None,
        discord_token: None,
        discord_enabled: None,
        slack_token: None,
        slack_enabled: None,
        whatsapp_bridge_url: None,
        whatsapp_api_token: None,
        whatsapp_enabled: None,
        signal_http_url: None,
        signal_account: None,
        signal_enabled: None,
    };
    if let Err(e) = state.config.write().await.update(update) {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
        })));
    }
    Ok(Json(serde_json::json!({
        "status": "switched",
        "model": model_id,
    })))
}

#[derive(Debug, Deserialize)]
struct CreateSkillRequest {
    name: String,
    description: String,
    content: String,
}

async fn list_skills() -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let skill_mgr = SkillManager::new();
    let list_str = skill_mgr.list();
    let parsed: serde_json::Value = match serde_json::from_str(&list_str) {
        Ok(v) => v,
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: format!("Failed to parse skills list: {}", e),
            })));
        }
    };
    let skills_array = parsed
        .get("skills")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let result: Vec<serde_json::Value> = skills_array
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "name": s["name"].as_str().unwrap_or(""),
                "description": s["description"].as_str().unwrap_or(""),
                "category": s.get("category").and_then(|v| v.as_str()).unwrap_or("general"),
                "enabled": s["enabled"].as_bool().unwrap_or(true),
            })
        })
        .collect();
    Ok(Json(serde_json::json!(result)))
}

async fn create_skill(
    Json(req): Json<CreateSkillRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut skill_mgr = SkillManager::new();
    let result = skill_mgr.create(&req.name, &req.description, &req.content);
    let parsed: serde_json::Value = serde_json::from_str(&result)
        .unwrap_or_else(|_| serde_json::json!({"raw": result}));
    if parsed.get("error").is_some() {
        Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: parsed["error"].as_str().unwrap_or("Unknown error").to_string(),
        })))
    } else {
        Ok(Json(parsed))
    }
}

async fn delete_skill(
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut skill_mgr = SkillManager::new();
    let result = skill_mgr.delete(&name);
    let parsed: serde_json::Value = serde_json::from_str(&result)
        .unwrap_or_else(|_| serde_json::json!({"raw": result}));
    if parsed.get("error").is_some() {
        Err((StatusCode::NOT_FOUND, Json(ErrorResponse {
            error: parsed["error"].as_str().unwrap_or("Unknown error").to_string(),
        })))
    } else {
        Ok(Json(parsed))
    }
}

async fn skills_growth() -> Json<serde_json::Value> {
    let skill_mgr = SkillManager::new();
    let list_str = skill_mgr.list();
    let skills: Vec<serde_json::Value> = serde_json::from_str(&list_str)
        .ok()
        .and_then(|v: serde_json::Value| v.get("skills").cloned())
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let mut by_date: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
    for skill in &skills {
        let date = skill.get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let name = skill.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        by_date.entry(date).or_default().push(name);
    }
    let mut result = Vec::new();
    let mut running = 0;
    for (date, names) in by_date {
        running += names.len();
        result.push(serde_json::json!({
            "date": date,
            "count": running,
            "new_skills": names,
        }));
    }
    Json(serde_json::json!({"skills_over_time": result}))
}