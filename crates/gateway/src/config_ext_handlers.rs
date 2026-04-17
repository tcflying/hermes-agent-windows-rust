use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::AppState;

type ApiError = (StatusCode, Json<crate::handlers::ErrorResponse>);

fn server_error(msg: impl std::fmt::Display) -> ApiError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(crate::handlers::ErrorResponse {
            error: msg.to_string(),
        }),
    )
}

fn bad_request(msg: impl std::fmt::Display) -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(crate::handlers::ErrorResponse {
            error: msg.to_string(),
        }),
    )
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ConfigDefaultsResponse {
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ConfigFieldInfo {
    pub r#type: String,
    pub default: serde_json::Value,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigSchemaResponse {
    pub fields: HashMap<String, ConfigFieldInfo>,
    pub category_order: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ConfigRawResponse {
    pub yaml: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfigRawUpdateRequest {
    pub yaml_text: String,
}

#[derive(Debug, Serialize)]
pub struct GenericOk {
    pub ok: bool,
}

// ---------------------------------------------------------------------------
// Schema definition — mirrors Config struct fields
// ---------------------------------------------------------------------------

fn build_schema() -> (HashMap<String, ConfigFieldInfo>, Vec<String>) {
    let mut fields = HashMap::new();

    fields.insert("model".into(), ConfigFieldInfo {
        r#type: "string".into(),
        default: serde_json::Value::String("MiniMax-M2.7-highspeed".into()),
        description: "Default model for conversations".into(),
    });
    fields.insert("provider".into(), ConfigFieldInfo {
        r#type: "string".into(),
        default: serde_json::Value::String("minimax".into()),
        description: "Default LLM provider (minimax, openai, anthropic, etc.)".into(),
    });
    fields.insert("api_url".into(), ConfigFieldInfo {
        r#type: "string".into(),
        default: serde_json::Value::String("https://api.minimaxi.com/v1".into()),
        description: "Base URL for the LLM API endpoint".into(),
    });
    fields.insert("api_key".into(), ConfigFieldInfo {
        r#type: "string".into(),
        default: serde_json::Value::String(String::new()),
        description: "API key (prefer .env file for secrets)".into(),
    });
    fields.insert("tools".into(), ConfigFieldInfo {
        r#type: "array".into(),
        default: serde_json::Value::Array(vec![]),
        description: "List of enabled tool names".into(),
    });
    fields.insert("display.skin".into(), ConfigFieldInfo {
        r#type: "string".into(),
        default: serde_json::Value::String("default".into()),
        description: "UI skin theme (default, ares, slate, mono)".into(),
    });
    fields.insert("platforms.telegram.bot_token".into(), ConfigFieldInfo {
        r#type: "string".into(),
        default: serde_json::Value::String(String::new()),
        description: "Telegram bot token from @BotFather".into(),
    });
    fields.insert("platforms.telegram.enabled".into(), ConfigFieldInfo {
        r#type: "boolean".into(),
        default: serde_json::Value::Bool(false),
        description: "Enable Telegram gateway adapter".into(),
    });
    fields.insert("platforms.discord.bot_token".into(), ConfigFieldInfo {
        r#type: "string".into(),
        default: serde_json::Value::String(String::new()),
        description: "Discord bot token".into(),
    });
    fields.insert("platforms.discord.enabled".into(), ConfigFieldInfo {
        r#type: "boolean".into(),
        default: serde_json::Value::Bool(false),
        description: "Enable Discord gateway adapter".into(),
    });
    fields.insert("platforms.slack.bot_token".into(), ConfigFieldInfo {
        r#type: "string".into(),
        default: serde_json::Value::String(String::new()),
        description: "Slack bot OAuth token".into(),
    });
    fields.insert("platforms.slack.enabled".into(), ConfigFieldInfo {
        r#type: "boolean".into(),
        default: serde_json::Value::Bool(false),
        description: "Enable Slack gateway adapter".into(),
    });
    fields.insert("platforms.whatsapp.bridge_url".into(), ConfigFieldInfo {
        r#type: "string".into(),
        default: serde_json::Value::String(String::new()),
        description: "WhatsApp bridge API URL".into(),
    });
    fields.insert("platforms.whatsapp.enabled".into(), ConfigFieldInfo {
        r#type: "boolean".into(),
        default: serde_json::Value::Bool(false),
        description: "Enable WhatsApp gateway adapter".into(),
    });
    fields.insert("platforms.signal.http_url".into(), ConfigFieldInfo {
        r#type: "string".into(),
        default: serde_json::Value::String(String::new()),
        description: "Signal CLI REST API URL".into(),
    });
    fields.insert("platforms.signal.enabled".into(), ConfigFieldInfo {
        r#type: "boolean".into(),
        default: serde_json::Value::Bool(false),
        description: "Enable Signal gateway adapter".into(),
    });
    fields.insert("platforms.feishu.app_id".into(), ConfigFieldInfo {
        r#type: "string".into(),
        default: serde_json::Value::String(String::new()),
        description: "Feishu/Lark App ID".into(),
    });
    fields.insert("platforms.feishu.app_secret".into(), ConfigFieldInfo {
        r#type: "string".into(),
        default: serde_json::Value::String(String::new()),
        description: "Feishu/Lark App Secret".into(),
    });
    fields.insert("platforms.feishu.verification_token".into(), ConfigFieldInfo {
        r#type: "string".into(),
        default: serde_json::Value::String(String::new()),
        description: "Feishu/Lark Event Verification Token".into(),
    });
    fields.insert("platforms.feishu.enabled".into(), ConfigFieldInfo {
        r#type: "boolean".into(),
        default: serde_json::Value::Bool(false),
        description: "Enable Feishu/Lark gateway adapter".into(),
    });

    let category_order = vec![
        "model".into(),
        "provider".into(),
        "api_url".into(),
        "api_key".into(),
        "tools".into(),
        "display.skin".into(),
        "platforms.telegram.bot_token".into(),
        "platforms.telegram.enabled".into(),
        "platforms.discord.bot_token".into(),
        "platforms.discord.enabled".into(),
        "platforms.slack.bot_token".into(),
        "platforms.slack.enabled".into(),
        "platforms.whatsapp.bridge_url".into(),
        "platforms.whatsapp.enabled".into(),
        "platforms.signal.http_url".into(),
        "platforms.signal.enabled".into(),
        "platforms.feishu.app_id".into(),
        "platforms.feishu.app_secret".into(),
        "platforms.feishu.verification_token".into(),
        "platforms.feishu.enabled".into(),
    ];

    (fields, category_order)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn get_defaults(
    State(_state): State<AppState>,
) -> Result<Json<ConfigDefaultsResponse>, ApiError> {
    let default_config = hermes_config::Config::default();
    let value = serde_json::to_value(&default_config).map_err(|e| server_error(e))?;
    Ok(Json(ConfigDefaultsResponse { config: value }))
}

pub async fn get_schema(
    State(_state): State<AppState>,
) -> Result<Json<ConfigSchemaResponse>, ApiError> {
    let (fields, category_order) = build_schema();
    Ok(Json(ConfigSchemaResponse {
        fields,
        category_order,
    }))
}

pub async fn get_raw(
    State(state): State<AppState>,
) -> Result<Json<ConfigRawResponse>, ApiError> {
    let config = state.config.read().await;
    let yaml = serde_yaml::to_string(config.get()).map_err(|e| server_error(e))?;
    Ok(Json(ConfigRawResponse { yaml }))
}

pub async fn put_raw(
    State(state): State<AppState>,
    Json(body): Json<ConfigRawUpdateRequest>,
) -> Result<Json<GenericOk>, ApiError> {
    if body.yaml_text.trim().is_empty() {
        return Err(bad_request("yaml_text must not be empty"));
    }

    let _parsed: hermes_config::Config =
        serde_yaml::from_str(&body.yaml_text).map_err(|e| bad_request(format!("Invalid YAML: {}", e)))?;

    let config = state.config.write().await;

    let path = std::env::var("HERMES_CONFIG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".hermes")
                .join("config.yaml")
        });

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| server_error(e))?;
    }
    std::fs::write(&path, &body.yaml_text).map_err(|e| server_error(e))?;

    drop(config);

    Ok(Json(GenericOk { ok: true }))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn config_ext_routes() -> Router<AppState> {
    Router::new()
        .route("/api/config/defaults", get(get_defaults))
        .route("/api/config/schema", get(get_schema))
        .route("/api/config/raw", get(get_raw).put(put_raw))
}
