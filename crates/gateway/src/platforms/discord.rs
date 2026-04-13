use super::{PlatformAdapter, SendMessage};
use async_trait::async_trait;

pub struct DiscordAdapter {
    bot_token: String,
    connected: bool,
    client: reqwest::Client,
}

impl DiscordAdapter {
    pub fn new(bot_token: &str) -> Self {
        Self {
            bot_token: bot_token.to_string(),
            connected: false,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PlatformAdapter for DiscordAdapter {
    fn name(&self) -> &str {
        "discord"
    }

    async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let resp = self.client
            .get("https://discord.com/api/v10/users/@me")
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await?;
        if resp.status().is_success() {
            self.connected = true;
            Ok(())
        } else {
            let body = resp.text().await?;
            Err(format!("Discord auth failed: {}", body).into())
        }
    }

    async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.connected = false;
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let body = serde_json::json!({
            "content": message.text
        });
        let resp = self.client
            .post(format!("https://discord.com/api/v10/channels/{}/messages", message.chat_id))
            .header("Authorization", format!("Bot {}", self.bot_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            let body = resp.text().await?;
            Err(format!("Discord send failed: {}", body).into())
        }
    }

    async fn send_image(&self, chat_id: &str, image_url: &str, caption: Option<&str>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut body = serde_json::json!({
            "embed": {
                "image": {"url": image_url}
            }
        });
        if let Some(cap) = caption {
            body["content"] = serde_json::Value::String(cap.to_string());
        }
        let resp = self.client
            .post(format!("https://discord.com/api/v10/channels/{}/messages", chat_id))
            .header("Authorization", format!("Bot {}", self.bot_token))
            .json(&body)
            .send()
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            let err = resp.text().await?;
            Err(format!("Discord send image failed: {}", err).into())
        }
    }

    async fn send_document(&self, chat_id: &str, file_url: &str, caption: Option<&str>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let body = serde_json::json!({
            "content": format!("{}{}", caption.unwrap_or(""), if caption.is_some() { "\n" } else { "" } )
        });
        let resp = self.client
            .post(format!("https://discord.com/api/v10/channels/{}/messages", chat_id))
            .header("Authorization", format!("Bot {}", self.bot_token))
            .json(&body)
            .send()
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            let err = resp.text().await?;
            Err(format!("Discord send document failed: {}", err).into())
        }
    }

    async fn send_typing(&self, chat_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _ = self.client
            .post(format!("https://discord.com/api/v10/channels/{}/typing", chat_id))
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await?;
        Ok(())
    }

    async fn edit_message(&self, chat_id: &str, message_id: &str, text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let body = serde_json::json!({"content": text});
        let resp = self.client
            .patch(format!("https://discord.com/api/v10/channels/{}/messages/{}", chat_id, message_id))
            .header("Authorization", format!("Bot {}", self.bot_token))
            .json(&body)
            .send()
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            let err = resp.text().await?;
            Err(format!("Discord edit failed: {}", err).into())
        }
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}
