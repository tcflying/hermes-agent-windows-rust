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
            self.config = serde_yaml::from_str(&content).unwrap_or_default();
        }
        Ok(())
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
            })
            .unwrap();
        assert_eq!(loader.get().model, "claude-sonnet-4-20250514");
    }
}
