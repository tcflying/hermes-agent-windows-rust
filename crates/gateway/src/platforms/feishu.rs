use super::{PlatformAdapter, SendMessage};
use async_trait::async_trait;
use std::sync::RwLock;
use std::time::Instant;

pub struct FeishuAdapter {
    app_id: String,
    app_secret: String,
    verification_token: String,
    base_url: String,
    connected: bool,
    token_cache: RwLock<TokenCache>,
    client: reqwest::Client,
}

struct TokenCache {
    token: String,
    expires_at: Instant,
}

impl FeishuAdapter {
    pub fn new(app_id: &str, app_secret: &str, verification_token: &str) -> Self {
        Self {
            app_id: app_id.to_string(),
            app_secret: app_secret.to_string(),
            verification_token: verification_token.to_string(),
            base_url: "https://open.feishu.cn".to_string(),
            connected: false,
            token_cache: RwLock::new(TokenCache {
                token: String::new(),
                expires_at: Instant::now(),
            }),
            client: reqwest::Client::new(),
        }
    }

    pub fn verification_token(&self) -> &str {
        &self.verification_token
    }

    async fn get_tenant_access_token(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        {
            let cache = self.token_cache.read().unwrap();
            if !cache.token.is_empty() && cache.expires_at > Instant::now() {
                return Ok(cache.token.clone());
            }
        }

        let resp = self.client
            .post(format!("{}/open-apis/auth/v3/tenant_access_token/internal", self.base_url))
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&serde_json::json!({
                "app_id": self.app_id,
                "app_secret": self.app_secret,
            }))
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;
        let token = body.get("tenant_access_token")
            .and_then(|v| v.as_str())
            .ok_or("Missing tenant_access_token in response")?
            .to_string();

        let expire = body.get("expire").and_then(|v| v.as_i64()).unwrap_or(7200);
        {
            let mut cache = self.token_cache.write().unwrap();
            cache.token = token.clone();
            cache.expires_at = Instant::now() + std::time::Duration::from_secs(expire as u64 - 300);
        }

        Ok(token)
    }

    pub async fn parse_event(&self, body: &serde_json::Value) -> Option<FeishuParsedEvent> {
        let event_type = body.get("header")
            .and_then(|h| h.get("event_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if event_type != "im.message.receive_v1" {
            return None;
        }

        let event = body.get("event")?;
        let sender = event.get("sender")?;
        let message = event.get("message")?;

        let user_id = sender.get("sender_id")
            .and_then(|id| id.get("open_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let chat_id = message.get("chat_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let chat_type = message.get("chat_type")
            .and_then(|v| v.as_str())
            .unwrap_or("p2p")
            .to_string();

        let message_id = message.get("message_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let msg_type = message.get("message_type")
            .and_then(|v| v.as_str())
            .unwrap_or("text")
            .to_string();

        let content_str = message.get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("{}");

        let text = if msg_type == "text" {
            serde_json::from_str::<serde_json::Value>(content_str)
                .ok()
                .and_then(|c| c.get("text").and_then(|t| t.as_str()).map(String::from))
                .unwrap_or_default()
        } else {
            format!("[{}]", msg_type)
        };

        Some(FeishuParsedEvent {
            text,
            user_id,
            chat_id,
            chat_type,
            message_id,
            event_id: body.get("header")
                .and_then(|h| h.get("event_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }

    pub async fn reply_message(&self, message_id: &str, text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token = self.get_tenant_access_token().await?;
        self.client
            .post(format!("{}/open-apis/im/v1/messages/{}/reply", self.base_url, message_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&serde_json::json!({
                "msg_type": "text",
                "content": serde_json::json!({"text": text}).to_string()
            }))
            .send()
            .await?;
        Ok(())
    }

    pub async fn send_to_chat(&self, chat_id: &str, text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token = self.get_tenant_access_token().await?;
        self.client
            .post(format!("{}/open-apis/im/v1/messages?receive_id_type=chat_id", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&serde_json::json!({
                "receive_id": chat_id,
                "msg_type": "text",
                "content": serde_json::json!({"text": text}).to_string()
            }))
            .send()
            .await?;
        Ok(())
    }
}

pub struct FeishuParsedEvent {
    pub text: String,
    pub user_id: String,
    pub chat_id: String,
    pub chat_type: String,
    pub message_id: String,
    pub event_id: String,
}

#[async_trait]
impl PlatformAdapter for FeishuAdapter {
    fn name(&self) -> &str {
        "feishu"
    }

    async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token = self.get_tenant_access_token().await?;
        if token.is_empty() {
            return Err("Failed to obtain tenant_access_token".into());
        }
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.connected = false;
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.send_to_chat(&message.chat_id, &message.text).await
    }

    async fn send_image(&self, chat_id: &str, image_url: &str, caption: Option<&str>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token = self.get_tenant_access_token().await?;
        let content = serde_json::json!({
            "image_key": image_url
        });
        let mut body = serde_json::json!({
            "receive_id": chat_id,
            "msg_type": "image",
            "content": content.to_string()
        });
        if let Some(cap) = caption {
            body["content"] = serde_json::json!({
                "image_key": image_url,
                "text": cap
            }).to_string().into();
        }
        self.client
            .post(format!("{}/open-apis/im/v1/messages?receive_id_type=chat_id", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&body)
            .send()
            .await?;
        Ok(())
    }

    async fn send_document(&self, chat_id: &str, file_path: &str, caption: Option<&str>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token = self.get_tenant_access_token().await?;
        let content = serde_json::json!({
            "file_key": file_path
        });
        let mut body = serde_json::json!({
            "receive_id": chat_id,
            "msg_type": "file",
            "content": content.to_string()
        });
        if let Some(cap) = caption {
            body["content"] = serde_json::json!({
                "file_key": file_path,
                "text": cap
            }).to_string().into();
        }
        self.client
            .post(format!("{}/open-apis/im/v1/messages?receive_id_type=chat_id", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&body)
            .send()
            .await?;
        Ok(())
    }

    async fn send_typing(&self, _chat_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    async fn edit_message(&self, _chat_id: &str, message_id: &str, text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token = self.get_tenant_access_token().await?;
        self.client
            .patch(format!("{}/open-apis/im/v1/messages/{}", self.base_url, message_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&serde_json::json!({
                "content": serde_json::json!({"text": text}).to_string()
            }))
            .send()
            .await?;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}
