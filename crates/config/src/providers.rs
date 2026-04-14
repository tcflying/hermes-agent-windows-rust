use serde::{Deserialize, Serialize};
use std::env;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub context_length: u32,
    #[serde(default = "default_true")]
    pub supports_tools: bool,
    #[serde(default = "default_true")]
    pub supports_streaming: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderDef {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key_env: String,
    pub models: Vec<ModelDef>,
}

pub fn all_providers() -> Vec<ProviderDef> {
    vec![
        ProviderDef {
            id: "minimax".into(),
            name: "MiniMax".into(),
            base_url: "https://api.minimaxi.com/v1".into(),
            api_key_env: "MINIMAX_API_KEY".into(),
            models: vec![
                ModelDef {
                    id: "MiniMax-M2.7-highspeed".into(),
                    name: "MiniMax M2.7 Highspeed".into(),
                    aliases: vec!["m2.7".into(), "minimax-fast".into()],
                    context_length: 131072,
                    supports_tools: true,
                    supports_streaming: true,
                },
                ModelDef {
                    id: "MiniMax-M2.5".into(),
                    name: "MiniMax M2.5".into(),
                    aliases: vec!["m2.5".into()],
                    context_length: 131072,
                    supports_tools: true,
                    supports_streaming: true,
                },
            ],
        },
        ProviderDef {
            id: "openai".into(),
            name: "OpenAI".into(),
            base_url: "https://api.openai.com/v1".into(),
            api_key_env: "OPENAI_API_KEY".into(),
            models: vec![
                ModelDef {
                    id: "gpt-4o".into(),
                    name: "GPT-4o".into(),
                    aliases: vec!["gpt4o".into()],
                    context_length: 128000,
                    supports_tools: true,
                    supports_streaming: true,
                },
                ModelDef {
                    id: "gpt-4o-mini".into(),
                    name: "GPT-4o Mini".into(),
                    aliases: vec!["gpt4o-mini".into()],
                    context_length: 128000,
                    supports_tools: true,
                    supports_streaming: true,
                },
                ModelDef {
                    id: "o3-mini".into(),
                    name: "O3 Mini".into(),
                    aliases: vec!["o3mini".into()],
                    context_length: 200000,
                    supports_tools: true,
                    supports_streaming: true,
                },
            ],
        },
        ProviderDef {
            id: "anthropic".into(),
            name: "Anthropic".into(),
            base_url: "https://api.anthropic.com".into(),
            api_key_env: "ANTHROPIC_API_KEY".into(),
            models: vec![
                ModelDef {
                    id: "claude-sonnet-4-20250514".into(),
                    name: "Claude Sonnet 4".into(),
                    aliases: vec!["sonnet4".into(), "claude-sonnet-4".into()],
                    context_length: 200000,
                    supports_tools: true,
                    supports_streaming: true,
                },
                ModelDef {
                    id: "claude-opus-4".into(),
                    name: "Claude Opus 4".into(),
                    aliases: vec!["opus4".into(), "claude-opus-4".into()],
                    context_length: 200000,
                    supports_tools: true,
                    supports_streaming: true,
                },
                ModelDef {
                    id: "claude-haiku-3.5".into(),
                    name: "Claude Haiku 3.5".into(),
                    aliases: vec!["haiku3.5".into(), "claude-haiku-3.5".into()],
                    context_length: 200000,
                    supports_tools: true,
                    supports_streaming: true,
                },
            ],
        },
        ProviderDef {
            id: "openrouter".into(),
            name: "OpenRouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            api_key_env: "OPENROUTER_API_KEY".into(),
            models: vec![],
        },
        ProviderDef {
            id: "google".into(),
            name: "Google".into(),
            base_url: "https://generativelanguage.googleapis.com/v1beta".into(),
            api_key_env: "GOOGLE_API_KEY".into(),
            models: vec![
                ModelDef {
                    id: "gemini-2.5-pro".into(),
                    name: "Gemini 2.5 Pro".into(),
                    aliases: vec!["gemini-pro".into()],
                    context_length: 1048576,
                    supports_tools: true,
                    supports_streaming: true,
                },
                ModelDef {
                    id: "gemini-2.5-flash".into(),
                    name: "Gemini 2.5 Flash".into(),
                    aliases: vec!["gemini-flash".into()],
                    context_length: 1048576,
                    supports_tools: true,
                    supports_streaming: true,
                },
            ],
        },
        ProviderDef {
            id: "zhipu".into(),
            name: "Zhipu AI".into(),
            base_url: "https://open.bigmodel.cn/api/paas/v4".into(),
            api_key_env: "ZHIPU_API_KEY".into(),
            models: vec![
                ModelDef {
                    id: "glm-5.1".into(),
                    name: "GLM 5.1".into(),
                    aliases: vec!["glm5".into()],
                    context_length: 128000,
                    supports_tools: true,
                    supports_streaming: true,
                },
                ModelDef {
                    id: "glm-4-plus".into(),
                    name: "GLM 4 Plus".into(),
                    aliases: vec!["glm4p".into()],
                    context_length: 128000,
                    supports_tools: true,
                    supports_streaming: true,
                },
            ],
        },
        ProviderDef {
            id: "deepseek".into(),
            name: "DeepSeek".into(),
            base_url: "https://api.deepseek.com/v1".into(),
            api_key_env: "DEEPSEEK_API_KEY".into(),
            models: vec![
                ModelDef {
                    id: "deepseek-chat".into(),
                    name: "DeepSeek Chat".into(),
                    aliases: vec!["ds-chat".into()],
                    context_length: 128000,
                    supports_tools: true,
                    supports_streaming: true,
                },
                ModelDef {
                    id: "deepseek-coder".into(),
                    name: "DeepSeek Coder".into(),
                    aliases: vec!["ds-coder".into()],
                    context_length: 128000,
                    supports_tools: true,
                    supports_streaming: true,
                },
            ],
        },
        ProviderDef {
            id: "moonshot".into(),
            name: "Moonshot".into(),
            base_url: "https://api.moonshot.cn/v1".into(),
            api_key_env: "MOONSHOT_API_KEY".into(),
            models: vec![ModelDef {
                id: "kimi-k2.5".into(),
                name: "Kimi K2.5".into(),
                aliases: vec!["kimi".into(), "k2.5".into()],
                context_length: 131072,
                supports_tools: true,
                supports_streaming: true,
            }],
        },
        ProviderDef {
            id: "mistral".into(),
            name: "Mistral".into(),
            base_url: "https://api.mistral.ai/v1".into(),
            api_key_env: "MISTRAL_API_KEY".into(),
            models: vec![
                ModelDef {
                    id: "mistral-large-latest".into(),
                    name: "Mistral Large".into(),
                    aliases: vec!["mistral-large".into()],
                    context_length: 128000,
                    supports_tools: true,
                    supports_streaming: true,
                },
                ModelDef {
                    id: "codestral-latest".into(),
                    name: "Codestral".into(),
                    aliases: vec!["codestral".into()],
                    context_length: 256000,
                    supports_tools: true,
                    supports_streaming: true,
                },
            ],
        },
        ProviderDef {
            id: "meta".into(),
            name: "Meta (via OpenRouter)".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            api_key_env: "OPENROUTER_API_KEY".into(),
            models: vec![ModelDef {
                id: "meta-llama/llama-3.3-70b-instruct".into(),
                name: "Llama 3.3 70B".into(),
                aliases: vec!["llama-3.3-70b".into(), "llama33".into()],
                context_length: 128000,
                supports_tools: true,
                supports_streaming: true,
            }],
        },
        ProviderDef {
            id: "alibaba".into(),
            name: "Alibaba Cloud".into(),
            base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".into(),
            api_key_env: "ALIBABA_API_KEY".into(),
            models: vec![
                ModelDef {
                    id: "qwen3.5-plus".into(),
                    name: "Qwen 3.5 Plus".into(),
                    aliases: vec!["qwen-plus".into()],
                    context_length: 131072,
                    supports_tools: true,
                    supports_streaming: true,
                },
                ModelDef {
                    id: "qwen3-max".into(),
                    name: "Qwen 3 Max".into(),
                    aliases: vec!["qwen-max".into()],
                    context_length: 32768,
                    supports_tools: true,
                    supports_streaming: true,
                },
            ],
        },
        ProviderDef {
            id: "vllm".into(),
            name: "qwen3".into(),
            base_url: "http://192.168.134.7:9191".into(),
            api_key_env: "VLLM_API_KEY".into(),
            models: vec![ModelDef {
                id: "Qwen/Qwen3.5-35B-A3B-GPTQ-Int4".into(),
                name: "Qwen 3.5 35B A3B GPTQ Int4".into(),
                aliases: vec!["oni".into()],
                context_length: 32768,
                supports_tools: true,
                supports_streaming: true,
            }],
        },
    ]
}

pub fn resolve_model(input: &str) -> Option<(String, String, String)> {
    let input_lower = input.to_lowercase();
    for provider in all_providers() {
        for model in &provider.models {
            for alias in &model.aliases {
                if alias.to_lowercase() == input_lower {
                    return Some((
                        provider.base_url.clone(),
                        provider.api_key_env.clone(),
                        model.id.clone(),
                    ));
                }
            }
        }
    }
    for provider in all_providers() {
        for model in &provider.models {
            if model.id.to_lowercase() == input_lower {
                return Some((
                    provider.base_url.clone(),
                    provider.api_key_env.clone(),
                    model.id.clone(),
                ));
            }
        }
    }
    None
}

pub fn detect_credentials() -> Vec<(String, bool)> {
    all_providers()
        .iter()
        .map(|p| {
            let has_key = env::var(&p.api_key_env)
                .map(|k| !k.is_empty())
                .unwrap_or(false);
            (p.id.clone(), has_key)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_providers_returns_nonempty() {
        let providers = all_providers();
        assert!(!providers.is_empty());
    }

    #[test]
    fn test_all_providers_has_minimax() {
        let providers = all_providers();
        assert!(providers.iter().any(|p| p.id == "minimax"));
    }

    #[test]
    fn test_resolve_model_by_id() {
        let result = resolve_model("MiniMax-M2.7-highspeed");
        assert!(result.is_some());
        let (base_url, _api_key_env, model_id) = result.unwrap();
        assert_eq!(base_url, "https://api.minimaxi.com/v1");
        assert_eq!(model_id, "MiniMax-M2.7-highspeed");
    }

    #[test]
    fn test_resolve_model_by_alias() {
        let result = resolve_model("m2.7");
        assert!(result.is_some());
        let (_base_url, _api_key_env, model_id) = result.unwrap();
        assert_eq!(model_id, "MiniMax-M2.7-highspeed");
    }

    #[test]
    fn test_resolve_model_case_insensitive() {
        let result = resolve_model("MINIMAX-M2.7-HIGHSPEED");
        assert!(result.is_some());
    }

    #[test]
    fn test_resolve_model_unknown_returns_none() {
        let result = resolve_model("nonexistent-model-xyz");
        assert!(result.is_none());
    }

    #[test]
    fn test_provider_def_has_models() {
        let providers_with_models = all_providers()
            .into_iter()
            .filter(|p| !p.models.is_empty())
            .count();
        assert!(providers_with_models > 0);
    }

    #[test]
    fn test_model_def_has_valid_base_url() {
        for provider in all_providers() {
            assert!(
                provider.base_url.starts_with("https://"),
                "provider '{}' base_url '{}' does not start with https://",
                provider.id,
                provider.base_url
            );
        }
    }
}
