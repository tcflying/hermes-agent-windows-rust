use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub api_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub display: DisplayConfig,
    #[serde(default)]
    pub platforms: PlatformConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlatformConfig {
    #[serde(default)]
    pub telegram: TelegramConfig,
    #[serde(default)]
    pub discord: DiscordConfig,
    #[serde(default)]
    pub slack: SlackConfig,
    #[serde(default)]
    pub whatsapp: WhatsAppConfig,
    #[serde(default)]
    pub signal: SignalConfig,
    #[serde(default)]
    pub feishu: FeishuConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelegramConfig {
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscordConfig {
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SlackConfig {
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    #[serde(default)]
    pub bridge_url: String,
    #[serde(default)]
    pub api_token: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignalConfig {
    #[serde(default)]
    pub http_url: String,
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeishuConfig {
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub app_secret: String,
    #[serde(default)]
    pub enabled: bool,
}

fn default_model() -> String {
    "MiniMax-M2.7-highspeed".to_string()
}

fn default_provider() -> String {
    "minimax".to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisplayConfig {
    #[serde(default = "default_skin")]
    pub skin: String,
}

fn default_skin() -> String {
    "default".to_string()
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            skin: default_skin(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: default_model(),
            provider: default_provider(),
            api_url: "https://api.minimaxi.com/v1".to_string(),
            api_key: String::new(),
            tools: vec![],
            display: DisplayConfig::default(),
            platforms: PlatformConfig::default(),
        }
    }
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            telegram: TelegramConfig::default(),
            discord: DiscordConfig::default(),
            slack: SlackConfig::default(),
            whatsapp: WhatsAppConfig::default(),
            signal: SignalConfig::default(),
            feishu: FeishuConfig::default(),
        }
    }
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            enabled: false,
        }
    }
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            enabled: false,
        }
    }
}

impl Default for SlackConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            enabled: false,
        }
    }
}

impl Default for WhatsAppConfig {
    fn default() -> Self {
        Self {
            bridge_url: String::new(),
            api_token: String::new(),
            enabled: false,
        }
    }
}

impl Default for SignalConfig {
    fn default() -> Self {
        Self {
            http_url: String::new(),
            account: String::new(),
            enabled: false,
        }
    }
}

impl Default for FeishuConfig {
    fn default() -> Self {
        Self {
            app_id: String::new(),
            app_secret: String::new(),
            enabled: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigUpdate {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub api_url: Option<String>,
    pub api_key: Option<String>,
    pub skin: Option<String>,
    pub telegram_token: Option<String>,
    pub telegram_enabled: Option<bool>,
    pub discord_token: Option<String>,
    pub discord_enabled: Option<bool>,
    pub slack_token: Option<String>,
    pub slack_enabled: Option<bool>,
    pub whatsapp_bridge_url: Option<String>,
    pub whatsapp_api_token: Option<String>,
    pub whatsapp_enabled: Option<bool>,
    pub signal_http_url: Option<String>,
    pub signal_account: Option<String>,
    pub signal_enabled: Option<bool>,
    pub feishu_app_id: Option<String>,
    pub feishu_app_secret: Option<String>,
    pub feishu_enabled: Option<bool>,
}

pub struct ConfigLoader {
    config: Config,
    path: Option<PathBuf>,
}

impl ConfigLoader {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            path: None,
        }
    }

    pub fn load(&mut self, path: PathBuf) -> Result<()> {
        self.path = Some(path.clone());
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            self.config = Self::parse_config_yaml(&content).unwrap_or_default();
        }
        Ok(())
    }

    fn parse_config_yaml(content: &str) -> Result<Config> {
        let value: serde_yaml::Value = serde_yaml::from_str(content)?;
        let mut clean: serde_yaml::Mapping = serde_yaml::Mapping::new();
        if let serde_yaml::Value::Mapping(map) = &value {
            for (k, v) in map {
                if !v.is_null() {
                    clean.insert(k.clone(), v.clone());
                }
            }
        }
        serde_yaml::from_value(serde_yaml::Value::Mapping(clean)).map_err(Into::into)
    }

    pub fn get(&self) -> &Config {
        &self.config
    }

    pub fn update(&mut self, update: ConfigUpdate) -> Result<()> {
        if let Some(m) = update.model {
            self.config.model = m;
        }
        if let Some(p) = update.provider {
            self.config.provider = p;
        }
        if let Some(u) = update.api_url {
            self.config.api_url = u;
        }
        if let Some(k) = update.api_key {
            self.config.api_key = k;
        }
        if let Some(s) = update.skin {
            self.config.display.skin = s;
        }
        if let Some(t) = update.telegram_token {
            self.config.platforms.telegram.bot_token = t;
        }
        if let Some(e) = update.telegram_enabled {
            self.config.platforms.telegram.enabled = e;
        }
        if let Some(t) = update.discord_token {
            self.config.platforms.discord.bot_token = t;
        }
        if let Some(e) = update.discord_enabled {
            self.config.platforms.discord.enabled = e;
        }
        if let Some(t) = update.slack_token {
            self.config.platforms.slack.bot_token = t;
        }
        if let Some(e) = update.slack_enabled {
            self.config.platforms.slack.enabled = e;
        }
        if let Some(u) = update.whatsapp_bridge_url {
            self.config.platforms.whatsapp.bridge_url = u;
        }
        if let Some(t) = update.whatsapp_api_token {
            self.config.platforms.whatsapp.api_token = t;
        }
        if let Some(e) = update.whatsapp_enabled {
            self.config.platforms.whatsapp.enabled = e;
        }
        if let Some(u) = update.signal_http_url {
            self.config.platforms.signal.http_url = u;
        }
        if let Some(a) = update.signal_account {
            self.config.platforms.signal.account = a;
        }
        if let Some(e) = update.signal_enabled {
            self.config.platforms.signal.enabled = e;
        }
        if let Some(id) = update.feishu_app_id {
            self.config.platforms.feishu.app_id = id;
        }
        if let Some(secret) = update.feishu_app_secret {
            self.config.platforms.feishu.app_secret = secret;
        }
        if let Some(e) = update.feishu_enabled {
            self.config.platforms.feishu.enabled = e;
        }
        self.save()
    }

    pub fn save(&self) -> Result<()> {
        if let Some(ref path) = self.path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let content = serde_yaml::to_string(&self.config)?;
            std::fs::write(path, content)?;
        }
        Ok(())
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default_model() {
        let config = Config::default();
        assert_eq!(config.model, "MiniMax-M2.7-highspeed");
    }

    #[test]
    fn test_config_default_provider() {
        let config = Config::default();
        assert_eq!(config.provider, "minimax");
    }

    #[test]
    fn test_config_default_api_url() {
        let config = Config::default();
        assert_eq!(config.api_url, "https://api.minimaxi.com/v1");
    }

    #[test]
    fn test_config_default_api_key_empty() {
        let config = Config::default();
        assert!(config.api_key.is_empty());
    }

    #[test]
    fn test_config_default_tools_empty() {
        let config = Config::default();
        assert!(config.tools.is_empty());
    }

    #[test]
    fn test_config_default_skin() {
        let config = Config::default();
        assert_eq!(config.display.skin, "default");
    }

    #[test]
    fn test_config_update_serialization() {
        let update = ConfigUpdate {
            model: Some("gpt-4o".to_string()),
            provider: Some("openai".to_string()),
            api_url: None,
            api_key: None,
            skin: Some("mono".to_string()),
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
            feishu_app_id: None,
            feishu_app_secret: None,
            feishu_enabled: None,
        };
        let yaml = serde_yaml::to_string(&update).unwrap();
        assert!(yaml.contains("gpt-4o"));
        assert!(yaml.contains("openai"));
        assert!(yaml.contains("mono"));
        let parsed: ConfigUpdate = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.model, Some("gpt-4o".to_string()));
        assert_eq!(parsed.provider, Some("openai".to_string()));
        assert!(parsed.api_url.is_none());
        assert!(parsed.api_key.is_none());
        assert_eq!(parsed.skin, Some("mono".to_string()));
    }

    #[test]
    fn test_config_loader_new_has_defaults() {
        let loader = ConfigLoader::new();
        let config = loader.get();
        assert_eq!(config.model, "MiniMax-M2.7-highspeed");
        assert_eq!(config.provider, "minimax");
    }

    #[test]
    fn test_config_loader_update_model() {
        let mut loader = ConfigLoader::new();
        loader.path = Some(std::env::temp_dir().join("hermes_test_config_update.yaml"));
        loader
            .update(ConfigUpdate {
                model: Some("claude-sonnet-4-20250514".to_string()),
                provider: None,
                api_url: None,
                api_key: None,
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
                feishu_app_id: None,
                feishu_app_secret: None,
                feishu_enabled: None,
            })
            .unwrap();
        assert_eq!(loader.get().model, "claude-sonnet-4-20250514");
    }
}
