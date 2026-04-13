use serde_json::json;
use std::fs;
use std::path::PathBuf;

pub struct AuxiliaryClient {
    providers: Vec<ProviderEndpoint>,
}

#[derive(Clone, Debug)]
pub struct ProviderEndpoint {
    pub name: String,
    pub api_url: String,
    pub api_key: String,
    pub model: String,
    pub capabilities: Vec<String>,
    pub priority: u32,
}

#[derive(Clone, Debug)]
pub enum AuxiliaryCapability {
    Vision,
    Summarization,
    Extraction,
}

impl AuxiliaryClient {
    pub fn new() -> Self {
        let mut providers = Vec::new();

        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            if !key.is_empty() {
                providers.push(ProviderEndpoint {
                    name: "anthropic".to_string(),
                    api_url: "https://api.anthropic.com/v1".to_string(),
                    api_key: key,
                    model: "claude-sonnet-4-5-20250514".to_string(),
                    capabilities: vec!["vision".to_string(), "summarization".to_string()],
                    priority: 1,
                });
            }
        }

        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            if !key.is_empty() {
                providers.push(ProviderEndpoint {
                    name: "openai".to_string(),
                    api_url: "https://api.openai.com/v1".to_string(),
                    api_key: key,
                    model: "gpt-4o".to_string(),
                    capabilities: vec!["vision".to_string(), "summarization".to_string(), "extraction".to_string()],
                    priority: 2,
                });
            }
        }

        if let Ok(key) = std::env::var("MINIMAX_API_KEY") {
            if !key.is_empty() {
                providers.push(ProviderEndpoint {
                    name: "minimax".to_string(),
                    api_url: "https://api.minimaxi.com/v1".to_string(),
                    api_key: key,
                    model: "MiniMax-M2.7-highspeed".to_string(),
                    capabilities: vec!["summarization".to_string(), "extraction".to_string()],
                    priority: 3,
                });
            }
        }

        providers.sort_by_key(|p| p.priority);
        Self { providers }
    }

    pub fn add_provider(&mut self, provider: ProviderEndpoint) {
        self.providers.push(provider);
        self.providers.sort_by_key(|p| p.priority);
    }

    fn get_best_provider(&self, capability: &str) -> Option<&ProviderEndpoint> {
        self.providers
            .iter()
            .find(|p| p.capabilities.contains(&capability.to_string()))
    }

    pub fn analyze_image(&self, image_path: &str, prompt: &str) -> String {
        let provider = match self.get_best_provider("vision") {
            Some(p) => p,
            None => return json!({"error": "No vision-capable provider available"}).to_string(),
        };

        let image_data = match fs::read(image_path) {
            Ok(data) => data,
            Err(e) => return json!({"error": format!("Failed to read image: {}", e)}).to_string(),
        };

        let b64 = base64_encode(&image_data);
        let path_buf = PathBuf::from(image_path);
        let ext = path_buf
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png");
        let mime = match ext {
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            _ => "image/png",
        };

        let data_url = format!("data:{};base64,{}", mime, b64);

        match provider.name.as_str() {
            "openai" => call_openai_vision(&provider, &data_url, prompt),
            "anthropic" => call_anthropic_vision(&provider, &data_url, prompt),
            _ => call_generic_vision(provider, &data_url, prompt),
        }
    }

    pub fn analyze_image_url(&self, url: &str, prompt: &str) -> String {
        let provider = match self.get_best_provider("vision") {
            Some(p) => p,
            None => return json!({"error": "No vision-capable provider available"}).to_string(),
        };

        match provider.name.as_str() {
            "openai" => call_openai_vision(provider, url, prompt),
            "anthropic" => call_anthropic_vision(provider, url, prompt),
            _ => call_generic_vision(provider, url, prompt),
        }
    }

    pub fn summarize_text(&self, text: &str, max_length: usize) -> String {
        let provider = match self.get_best_provider("summarization") {
            Some(p) => p,
            None => return json!({"error": "No summarization provider available"}).to_string(),
        };

        let system = format!("Summarize the following text in at most {} characters. Be concise and capture key points.", max_length);
        call_text_completion(provider, &system, text)
    }

    pub fn extract_entities(&self, text: &str) -> String {
        let provider = match self.get_best_provider("extraction") {
            Some(p) => p,
            None => return json!({"error": "No extraction provider available"}).to_string(),
        };

        let system = "Extract named entities from the following text. Return a JSON object with categories: persons, organizations, locations, dates, and other notable entities.";
        call_text_completion(provider, system, text)
    }
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let chunks = data.chunks(3);
    for chunk in chunks {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn call_openai_vision(provider: &ProviderEndpoint, image_url: &str, prompt: &str) -> String {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();

    match rt {
        Ok(runtime) => {
            let result = runtime.block_on(async {
                let client = reqwest::Client::new();
                let body = serde_json::json!({
                    "model": provider.model,
                    "messages": [{
                        "role": "user",
                        "content": [
                            {"type": "text", "text": prompt},
                            {"type": "image_url", "image_url": {"url": image_url}}
                        ]
                    }],
                    "max_tokens": 4096
                });

                let resp = client
                    .post(format!("{}/chat/completions", provider.api_url))
                    .header("Authorization", format!("Bearer {}", provider.api_key))
                    .json(&body)
                    .send()
                    .await;

                match resp {
                    Ok(r) => match r.text().await {
                        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
                            Ok(val) => {
                                let content = val.pointer("/choices/0/message/content")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                json!({"status": "ok", "provider": provider.name, "analysis": content})
                            }
                            Err(e) => json!({"error": format!("Parse: {}", e), "raw": text.chars().take(500).collect::<String>()}),
                        },
                        Err(e) => json!({"error": format!("Body: {}", e)}),
                    },
                    Err(e) => json!({"error": format!("Request: {}", e)}),
                }
            });
            result.to_string()
        }
        Err(e) => json!({"error": format!("Runtime: {}", e)}).to_string(),
    }
}

fn call_anthropic_vision(provider: &ProviderEndpoint, image_url: &str, prompt: &str) -> String {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();

    match rt {
        Ok(runtime) => {
            let result = runtime.block_on(async {
                let client = reqwest::Client::new();

                let image_content = if image_url.starts_with("data:") {
                    let parts: Vec<&str> = image_url.splitn(2, ',').collect();
                    json!({
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": parts.get(0).unwrap_or(&"").split(';').next().unwrap_or("").replace("data:", ""),
                            "data": parts.get(1).unwrap_or(&"")
                        }
                    })
                } else {
                    json!({
                        "type": "image",
                        "source": {"type": "url", "url": image_url}
                    })
                };

                let body = serde_json::json!({
                    "model": provider.model,
                    "max_tokens": 4096,
                    "messages": [{
                        "role": "user",
                        "content": [
                            {"type": "text", "text": prompt},
                            image_content
                        ]
                    }]
                });

                let resp = client
                    .post(format!("{}/messages", provider.api_url))
                    .header("x-api-key", &provider.api_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&body)
                    .send()
                    .await;

                match resp {
                    Ok(r) => match r.text().await {
                        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
                            Ok(val) => {
                                let content = val.pointer("/content/0/text")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                json!({"status": "ok", "provider": provider.name, "analysis": content})
                            }
                            Err(e) => json!({"error": format!("Parse: {}", e)}),
                        },
                        Err(e) => json!({"error": format!("Body: {}", e)}),
                    },
                    Err(e) => json!({"error": format!("Request: {}", e)}),
                }
            });
            result.to_string()
        }
        Err(e) => json!({"error": format!("Runtime: {}", e)}).to_string(),
    }
}

fn call_generic_vision(provider: &ProviderEndpoint, image_url: &str, prompt: &str) -> String {
    call_openai_vision(provider, image_url, prompt)
}

fn call_text_completion(provider: &ProviderEndpoint, system: &str, user_text: &str) -> String {
    let truncated = if user_text.len() > 50000 {
        &user_text[..50000]
    } else {
        user_text
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();

    match rt {
        Ok(runtime) => {
            let result = runtime.block_on(async {
                let client = reqwest::Client::new();
                let body = serde_json::json!({
                    "model": provider.model,
                    "messages": [
                        {"role": "system", "content": system},
                        {"role": "user", "content": truncated}
                    ],
                    "max_tokens": 4096
                });

                let resp = client
                    .post(format!("{}/chat/completions", provider.api_url))
                    .header("Authorization", format!("Bearer {}", provider.api_key))
                    .json(&body)
                    .send()
                    .await;

                match resp {
                    Ok(r) => match r.text().await {
                        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
                            Ok(val) => {
                                let content = val.pointer("/choices/0/message/content")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                json!({"status": "ok", "provider": provider.name, "result": content})
                            }
                            Err(e) => json!({"error": format!("Parse: {}", e)}),
                        },
                        Err(e) => json!({"error": format!("Body: {}", e)}),
                    },
                    Err(e) => json!({"error": format!("Request: {}", e)}),
                }
            });
            result.to_string()
        }
        Err(e) => json!({"error": format!("Runtime: {}", e)}).to_string(),
    }
}
