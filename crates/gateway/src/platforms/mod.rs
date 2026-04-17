pub mod discord;
pub mod feishu;
pub mod signal;
pub mod slack;
pub mod telegram;
pub mod whatsapp;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEvent {
    pub text: String,
    pub source: String,
    pub user_id: String,
    pub chat_id: String,
    pub chat_type: String,
    pub reply_to: Option<String>,
    pub media: Option<MediaAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAttachment {
    pub media_type: String,
    pub url: String,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessage {
    pub text: String,
    pub chat_id: String,
    pub parse_mode: Option<String>,
    pub reply_to: Option<String>,
}

#[async_trait]
pub trait PlatformAdapter: Send + Sync {
    fn name(&self) -> &str;
    async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn send(&self, message: SendMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn send_image(&self, chat_id: &str, image_url: &str, caption: Option<&str>) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn send_document(&self, chat_id: &str, file_path: &str, caption: Option<&str>) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn send_typing(&self, chat_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn edit_message(&self, chat_id: &str, message_id: &str, text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn is_connected(&self) -> bool;
}
