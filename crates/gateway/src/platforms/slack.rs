use super::{PlatformAdapter, SendMessage};
use async_trait::async_trait;

pub struct SlackAdapter {
    bot_token: String,
    connected: bool,
    client: reqwest::Client,
}

impl SlackAdapter {
    pub fn new(bot_token: &str) -> Self {
        Self {
            bot_token: bot_token.to_string(),
            connected: false,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PlatformAdapter for SlackAdapter {
    fn name(&self) -> &str {
        "slack"
    }

    async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let resp = self.client
            .post("https://slack.com/api/auth.test")
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .send()
            .await?;
        let body: serde_json::Value = resp.json().await?;
        if body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            self.connected = true;
            Ok(())
        } else {
            Err(format!("Slack auth.test failed: {}", body).into())
        }
    }

    async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.connected = false;
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut body = serde_json::json!({
            "channel": message.chat_id,
            "text": message.text,
        });
        if let Some(reply_to) = message.reply_to {
            body["thread_ts"] = serde_json::Value::String(reply_to);
        }
        let resp = self.client
            .post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;
        let result: serde_json::Value = resp.json().await?;
        if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            Err(format!("chat.postMessage failed: {}", result).into())
        }
    }

    async fn send_image(&self, chat_id: &str, image_url: &str, caption: Option<&str>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut body = serde_json::json!({
            "channel": chat_id,
            "blocks": [{
                "type": "image",
                "image_url": image_url,
                "alt_text": caption.unwrap_or("image")
            }]
        });
        if let Some(cap) = caption {
            body["text"] = serde_json::Value::String(cap.to_string());
        }
        let resp = self.client
            .post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .json(&body)
            .send()
            .await?;
        let result: serde_json::Value = resp.json().await?;
        if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            Err(format!("send image failed: {}", result).into())
        }
    }

    async fn send_document(&self, chat_id: &str, file_url: &str, caption: Option<&str>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let body = serde_json::json!({
            "channel": chat_id,
            "text": caption.unwrap_or("Document"),
            "blocks": [{
                "type": "section",
                "text": {"type": "mrkdwn", "text": format!("<{}|{}>", file_url, caption.unwrap_or("Download file"))}
            }]
        });
        let resp = self.client
            .post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .json(&body)
            .send()
            .await?;
        let result: serde_json::Value = resp.json().await?;
        if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            Err(format!("send document failed: {}", result).into())
        }
    }

    async fn send_typing(&self, _chat_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    async fn edit_message(&self, chat_id: &str, message_id: &str, text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let body = serde_json::json!({
            "channel": chat_id,
            "ts": message_id,
            "text": text
        });
        let resp = self.client
            .post("https://slack.com/api/chat.update")
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .json(&body)
            .send()
            .await?;
        let result: serde_json::Value = resp.json().await?;
        if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            Err(format!("chat.update failed: {}", result).into())
        }
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}
