use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, RwLock};

use super::{PlatformAdapter, SendMessage};

const FEISHU_BASE_URL: &str = "https://open.feishu.cn";
const TOKEN_REFRESH_MARGIN_SECS: u64 = 300;

const FRAME_METHOD_CONTROL: i32 = 0;
const FRAME_METHOD_DATA: i32 = 1;

#[derive(Clone, PartialEq, ProstMessage)]
struct Frame {
    #[prost(uint64, tag = 1)]
    seq_id: u64,
    #[prost(uint64, tag = 2)]
    log_id: u64,
    #[prost(int32, tag = 3)]
    service: i32,
    #[prost(int32, tag = 4)]
    method: i32,
    #[prost(message, repeated, tag = 5)]
    headers: Vec<FrameHeader>,
    #[prost(string, tag = 6)]
    payload_encoding: String,
    #[prost(string, tag = 7)]
    payload_type: String,
    #[prost(bytes = "vec", tag = 8)]
    payload: Vec<u8>,
    #[prost(string, tag = 9)]
    log_id_new: String,
}

#[derive(Clone, PartialEq, ProstMessage)]
struct FrameHeader {
    #[prost(string, tag = 1)]
    key: String,
    #[prost(string, tag = 2)]
    value: String,
}

impl Frame {
    fn get_header(&self, key: &str) -> Option<&str> {
        self.headers.iter().find(|h| h.key == key).map(|h| h.value.as_str())
    }

    fn set_header(&mut self, key: &str, value: &str) {
        if let Some(h) = self.headers.iter_mut().find(|h| h.key == key) {
            h.value = value.to_string();
        } else {
            self.headers.push(FrameHeader {
                key: key.to_string(),
                value: value.to_string(),
            });
        }
    }
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    code: i32,
    msg: String,
    tenant_access_token: Option<String>,
    expire: Option<u64>,
}

#[derive(serde::Deserialize)]
struct WsEndpointResponse {
    code: i32,
    msg: String,
    data: Option<WsEndpointData>,
}

#[derive(serde::Deserialize)]
struct WsEndpointData {
    #[serde(rename = "URL")]
    url: Option<String>,
}

struct TokenCache {
    token: String,
    expires_at: Instant,
}

#[derive(Debug, Clone)]
pub struct FeishuInboundMessage {
    pub text: String,
    pub sender_id: String,
    pub chat_id: String,
    pub chat_type: String,
    pub message_id: String,
}

pub struct FeishuAdapter {
    app_id: String,
    app_secret: String,
    client: reqwest::Client,
    token_cache: Arc<RwLock<Option<TokenCache>>>,
    connected: Arc<Mutex<bool>>,
    shutdown_tx: Arc<Mutex<Option<tokio::sync::watch::Sender<bool>>>>,
    message_tx: Arc<Mutex<Option<mpsc::Sender<FeishuInboundMessage>>>>,
}

impl FeishuAdapter {
    pub fn new(app_id: &str, app_secret: &str) -> Self {
        Self {
            app_id: app_id.to_string(),
            app_secret: app_secret.to_string(),
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            token_cache: Arc::new(RwLock::new(None)),
            connected: Arc::new(Mutex::new(false)),
            shutdown_tx: Arc::new(Mutex::new(None)),
            message_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn get_tenant_access_token(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        {
            let cache = self.token_cache.read().await;
            if let Some(cached) = cache.as_ref() {
                if cached.expires_at > Instant::now() {
                    return Ok(cached.token.clone());
                }
            }
        }

        let resp = self.client
            .post(format!("{}/open-apis/auth/v3/tenant_access_token/internal", FEISHU_BASE_URL))
            .json(&serde_json::json!({
                "app_id": self.app_id,
                "app_secret": self.app_secret,
            }))
            .send()
            .await?;

        let result: TokenResponse = resp.json().await?;
        if result.code != 0 {
            return Err(format!("token error: code={}, msg={}", result.code, result.msg).into());
        }

        let token = result.tenant_access_token.ok_or("No token in response")?;
        let expire_secs = result.expire.unwrap_or(7200);
        let expires_at = Instant::now() + Duration::from_secs(expire_secs.saturating_sub(TOKEN_REFRESH_MARGIN_SECS));

        *self.token_cache.write().await = Some(TokenCache { token: token.clone(), expires_at });
        Ok(token)
    }

    pub async fn start_ws(&self) -> Result<mpsc::Receiver<FeishuInboundMessage>, Box<dyn std::error::Error + Send + Sync>> {
        self.get_tenant_access_token().await?;
        let _ = self.fetch_ws_endpoint().await?;

        let (tx, rx) = mpsc::channel::<FeishuInboundMessage>(256);
        *self.message_tx.lock().await = Some(tx);

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let app_id = self.app_id.clone();
        let app_secret = self.app_secret.clone();
        let client = self.client.clone();
        let connected = self.connected.clone();
        let message_tx = self.message_tx.clone();

        tokio::spawn(async move {
            let mut retry_count = 0u32;
            loop {
                if *shutdown_rx.borrow() { break; }

                let ws_url = match fetch_ws_url(&client, &app_id, &app_secret).await {
                    Ok(url) => url,
                    Err(e) => {
                        eprintln!("\x1b[31m[feishu] failed to get WS endpoint: {e}\x1b[0m");
                        *connected.lock().await = false;
                        let delay = Duration::from_secs(2u64.pow(retry_count.min(5)));
                        retry_count += 1;
                        tokio::select! {
                            _ = tokio::time::sleep(delay) => continue,
                            _ = shutdown_rx.changed() => break,
                        }
                    }
                };

                let ws_stream = match tokio_tungstenite::connect_async(&ws_url).await {
                    Ok((stream, _)) => {
                        *connected.lock().await = true;
                        retry_count = 0;
                        print!("\x1b[32m  ✓ Feishu WebSocket connected\x1b[0m\n");
                        stream
                    }
                    Err(e) => {
                        eprintln!("\x1b[31m[feishu] WS connect failed: {e}\x1b[0m");
                        *connected.lock().await = false;
                        let delay = Duration::from_secs(2u64.pow(retry_count.min(5)));
                        retry_count += 1;
                        tokio::select! {
                            _ = tokio::time::sleep(delay) => continue,
                            _ = shutdown_rx.changed() => break,
                        }
                    }
                };

                let (mut write, mut read) = ws_stream.split();

                loop {
                    tokio::select! {
                        msg = read.next() => {
                            match msg {
                                Some(Ok(tokio_tungstenite::tungstenite::Message::Binary(data))) => {
                                    match Frame::decode(data.as_ref()) {
                                        Ok(frame) => {
                                            let frame_type = frame.get_header("type").unwrap_or("").to_string();

                                            if frame.method == FRAME_METHOD_CONTROL && frame_type == "ping" {
                                                let mut pong = frame.clone();
                                                pong.set_header("type", "pong");
                                                pong.payload = Vec::new();
                                                let mut buf = Vec::new();
                                                if pong.encode(&mut buf).is_ok() {
                                                    let _ = write.send(tokio_tungstenite::tungstenite::Message::Binary(buf.into())).await;
                                                }
                                            } else if frame.method == FRAME_METHOD_DATA && frame_type == "event" {
                                                let start = Instant::now();
                                                if let Ok(payload_str) = std::str::from_utf8(&frame.payload) {
                                                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(payload_str) {
                                                        if let Some(inbound) = parse_message_event(&event) {
                                                            let preview = if inbound.text.len() > 60 {
                                                                format!("{}...", &inbound.text[..inbound.text.ceil_char_boundary(60)])
                                                            } else {
                                                                inbound.text.clone()
                                                            };
                                                            print!("\x1b[35m[feishu] message from {} in {}: \"{}\"\x1b[0m\n", inbound.sender_id, inbound.chat_type, preview);
                                                            if let Some(tx) = message_tx.lock().await.as_ref() {
                                                                let _ = tx.send(inbound).await;
                                                            }
                                                        }
                                                    }
                                                }

                                                let elapsed_ms = start.elapsed().as_millis();
                                                let mut ack = frame.clone();
                                                ack.payload = br#"{"code":200}"#.to_vec();
                                                ack.set_header("biz_rt", &elapsed_ms.to_string());
                                                let mut buf = Vec::new();
                                                if ack.encode(&mut buf).is_ok() {
                                                    let _ = write.send(tokio_tungstenite::tungstenite::Message::Binary(buf.into())).await;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("\x1b[33m[feishu] protobuf decode error: {e}\x1b[0m");
                                        }
                                    }
                                }
                                Some(Ok(tokio_tungstenite::tungstenite::Message::Ping(data))) => {
                                    let _ = write.send(tokio_tungstenite::tungstenite::Message::Pong(data)).await;
                                }
                                Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) | None => {
                                    eprintln!("\x1b[33m[feishu] WebSocket closed, reconnecting...\x1b[0m");
                                    break;
                                }
                                Some(Err(e)) => {
                                    eprintln!("\x1b[31m[feishu] WebSocket error: {e}\x1b[0m");
                                    break;
                                }
                                _ => {}
                            }
                        }
                        _ = shutdown_rx.changed() => {
                            let _ = write.close().await;
                            *connected.lock().await = false;
                            return;
                        }
                    }
                }

                *connected.lock().await = false;
                let delay = Duration::from_secs(3);
                tokio::select! {
                    _ = tokio::time::sleep(delay) => {},
                    _ = shutdown_rx.changed() => break,
                }
            }
            *connected.lock().await = false;
        });

        Ok(rx)
    }

    pub async fn stop_ws(&self) {
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(true);
        }
        *self.connected.lock().await = false;
    }

    pub async fn reply_message(&self, message_id: &str, text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token = self.get_tenant_access_token().await?;
        self.client
            .post(format!("{}/open-apis/im/v1/messages/{}/reply", FEISHU_BASE_URL, message_id))
            .header("Authorization", format!("Bearer {}", token))
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
            .post(format!("{}/open-apis/im/v1/messages?receive_id_type=chat_id", FEISHU_BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({
                "receive_id": chat_id,
                "msg_type": "text",
                "content": serde_json::json!({"text": text}).to_string()
            }))
            .send()
            .await?;
        Ok(())
    }

    async fn fetch_ws_endpoint(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        fetch_ws_url(&self.client, &self.app_id, &self.app_secret).await
    }
}

fn parse_message_event(event: &serde_json::Value) -> Option<FeishuInboundMessage> {
    let event_type = event.pointer("/header/event_type").and_then(|v| v.as_str()).unwrap_or("");
    if event_type != "im.message.receive_v1" {
        return None;
    }

    let msg_type = event.pointer("/event/message/message_type").and_then(|v| v.as_str()).unwrap_or("");
    if msg_type != "text" {
        return None;
    }

    let chat_type = event.pointer("/event/message/chat_type").and_then(|v| v.as_str()).unwrap_or("p2p");
    if chat_type == "group" {
        let mentions = event.pointer("/event/message/mentions").and_then(|v| v.as_array());
        if mentions.is_none() || mentions.unwrap().is_empty() {
            return None;
        }
    }

    let content_str = event.pointer("/event/message/content").and_then(|v| v.as_str()).unwrap_or("{}");
    let mut text = serde_json::from_str::<serde_json::Value>(content_str)
        .ok()
        .and_then(|v| v.get("text").and_then(|t| t.as_str()).map(String::from))
        .unwrap_or_default();

    if let Some(mentions) = event.pointer("/event/message/mentions").and_then(|v| v.as_array()) {
        for mention in mentions {
            if let Some(key) = mention.get("key").and_then(|v| v.as_str()) {
                text = text.replace(key, "");
            }
        }
    }
    let text = text.trim().to_string();
    if text.is_empty() {
        return None;
    }

    Some(FeishuInboundMessage {
        text,
        sender_id: event.pointer("/event/sender/sender_id/open_id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
        chat_id: event.pointer("/event/message/chat_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        chat_type: chat_type.to_string(),
        message_id: event.pointer("/event/message/message_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    })
}

async fn fetch_ws_url(client: &reqwest::Client, app_id: &str, app_secret: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let resp = client
        .post(format!("{}/callback/ws/endpoint", FEISHU_BASE_URL))
        .json(&serde_json::json!({ "AppID": app_id, "AppSecret": app_secret }))
        .send()
        .await?;

    let ws_resp: WsEndpointResponse = resp.json().await?;
    if ws_resp.code != 0 {
        return Err(format!("WS endpoint error: code={}, msg={}", ws_resp.code, ws_resp.msg).into());
    }
    ws_resp.data.and_then(|d| d.url).ok_or_else(|| "No WebSocket URL returned".into())
}

#[async_trait]
impl PlatformAdapter for FeishuAdapter {
    fn name(&self) -> &str { "feishu" }

    async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token = self.get_tenant_access_token().await?;
        if token.is_empty() {
            return Err("Failed to obtain tenant_access_token".into());
        }
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.stop_ws().await;
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.send_to_chat(&message.chat_id, &message.text).await
    }

    async fn send_image(&self, chat_id: &str, image_url: &str, _caption: Option<&str>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token = self.get_tenant_access_token().await?;
        self.client
            .post(format!("{}/open-apis/im/v1/messages?receive_id_type=chat_id", FEISHU_BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({ "receive_id": chat_id, "msg_type": "image", "content": serde_json::json!({"image_key": image_url}).to_string() }))
            .send()
            .await?;
        Ok(())
    }

    async fn send_document(&self, chat_id: &str, file_path: &str, _caption: Option<&str>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token = self.get_tenant_access_token().await?;
        self.client
            .post(format!("{}/open-apis/im/v1/messages?receive_id_type=chat_id", FEISHU_BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({ "receive_id": chat_id, "msg_type": "file", "content": serde_json::json!({"file_key": file_path}).to_string() }))
            .send()
            .await?;
        Ok(())
    }

    async fn send_typing(&self, _chat_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> { Ok(()) }

    async fn edit_message(&self, _chat_id: &str, message_id: &str, text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token = self.get_tenant_access_token().await?;
        self.client
            .patch(format!("{}/open-apis/im/v1/messages/{}", FEISHU_BASE_URL, message_id))
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({ "content": serde_json::json!({"text": text}).to_string() }))
            .send()
            .await?;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        false
    }
}
