use super::{MediaAttachment, MessageEvent, PlatformAdapter, SendMessage};
use async_trait::async_trait;

pub struct TelegramAdapter {
    bot_token: String,
    api_base: String,
    connected: bool,
    last_update_id: i64,
    client: reqwest::Client,
}

impl TelegramAdapter {
    pub fn new(bot_token: &str) -> Self {
        Self {
            bot_token: bot_token.to_string(),
            api_base: format!("https://api.telegram.org/bot{}", bot_token),
            connected: false,
            last_update_id: 0,
            client: reqwest::Client::new(),
        }
    }

    pub async fn poll_updates(&mut self) -> Result<Vec<MessageEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/getUpdates?offset={}&timeout=30", self.api_base, self.last_update_id + 1);
        let resp = self.client.get(&url).send().await?;
        let body: serde_json::Value = resp.json().await?;

        let mut events = Vec::new();
        if let Some(results) = body.get("result").and_then(|r| r.as_array()) {
            for update in results {
                if let Some(update_id) = update.get("update_id").and_then(|v| v.as_i64()) {
                    self.last_update_id = update_id;
                }

                if let Some(message) = update.get("message") {
                    let text = message.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let chat_id = message.get("chat")
                        .and_then(|c| c.get("id"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0)
                        .to_string();
                    let user_id = message.get("from")
                        .and_then(|f| f.get("id"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0)
                        .to_string();
                    let chat_type = message.get("chat")
                        .and_then(|c| c.get("type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("private")
                        .to_string();
                    let reply_to = message.get("reply_to_message")
                        .and_then(|r| r.get("message_id"))
                        .and_then(|v| v.as_i64())
                        .map(|v| v.to_string());

                    let media = if let Some(photo) = message.get("photo").and_then(|v| v.as_array()) {
                        if let Some(largest) = photo.last() {
                            let file_id = largest.get("file_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            Some(MediaAttachment {
                                media_type: "photo".to_string(),
                                url: file_id,
                                mime_type: Some("image/jpeg".to_string()),
                            })
                        } else {
                            None
                        }
                    } else if let Some(voice) = message.get("voice") {
                        let file_id = voice.get("file_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        Some(MediaAttachment {
                            media_type: "voice".to_string(),
                            url: file_id,
                            mime_type: voice.get("mime_type").and_then(|v| v.as_str()).map(String::from),
                        })
                    } else if let Some(doc) = message.get("document") {
                        let file_id = doc.get("file_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        Some(MediaAttachment {
                            media_type: "document".to_string(),
                            url: file_id,
                            mime_type: doc.get("mime_type").and_then(|v| v.as_str()).map(String::from),
                        })
                    } else {
                        None
                    };

                    if !text.is_empty() || media.is_some() {
                        events.push(MessageEvent {
                            text,
                            source: "telegram".to_string(),
                            user_id,
                            chat_id,
                            chat_type,
                            reply_to,
                            media,
                        });
                    }
                }
            }
        }
        Ok(events)
    }

    pub async fn get_me(&self) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/getMe", self.api_base);
        let resp = self.client.get(&url).send().await?;
        let body: serde_json::Value = resp.json().await?;
        Ok(body)
    }
}

#[async_trait]
impl PlatformAdapter for TelegramAdapter {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let result = self.get_me().await?;
        if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            self.connected = true;
            Ok(())
        } else {
            Err(format!("Telegram getMe failed: {}", result).into())
        }
    }

    async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.connected = false;
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/sendMessage", self.api_base);
        let mut body = serde_json::json!({
            "chat_id": message.chat_id,
            "text": message.text,
        });
        if let Some(parse_mode) = message.parse_mode {
            body["parse_mode"] = serde_json::Value::String(parse_mode);
        }
        if let Some(reply_to) = message.reply_to {
            body["reply_to_message_id"] = serde_json::Value::String(reply_to);
        }
        let resp = self.client.post(&url).json(&body).send().await?;
        let result: serde_json::Value = resp.json().await?;
        if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            Err(format!("sendMessage failed: {}", result).into())
        }
    }

    async fn send_image(&self, chat_id: &str, image_url: &str, caption: Option<&str>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/sendPhoto", self.api_base);
        let mut body = serde_json::json!({
            "chat_id": chat_id,
            "photo": image_url,
        });
        if let Some(cap) = caption {
            body["caption"] = serde_json::Value::String(cap.to_string());
        }
        let resp = self.client.post(&url).json(&body).send().await?;
        let result: serde_json::Value = resp.json().await?;
        if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            Err(format!("sendPhoto failed: {}", result).into())
        }
    }

    async fn send_document(&self, chat_id: &str, file_url: &str, caption: Option<&str>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/sendDocument", self.api_base);
        let mut body = serde_json::json!({
            "chat_id": chat_id,
            "document": file_url,
        });
        if let Some(cap) = caption {
            body["caption"] = serde_json::Value::String(cap.to_string());
        }
        let resp = self.client.post(&url).json(&body).send().await?;
        let result: serde_json::Value = resp.json().await?;
        if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            Err(format!("sendDocument failed: {}", result).into())
        }
    }

    async fn send_typing(&self, chat_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/sendChatAction", self.api_base);
        let body = serde_json::json!({
            "chat_id": chat_id,
            "action": "typing"
        });
        let _ = self.client.post(&url).json(&body).send().await?;
        Ok(())
    }

    async fn edit_message(&self, chat_id: &str, message_id: &str, text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/editMessageText", self.api_base);
        let body = serde_json::json!({
            "chat_id": chat_id,
            "message_id": message_id,
            "text": text
        });
        let resp = self.client.post(&url).json(&body).send().await?;
        let result: serde_json::Value = resp.json().await?;
        if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            Err(format!("editMessageText failed: {}", result).into())
        }
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}
