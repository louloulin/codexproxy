use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub providers: ProvidersConfig,
    pub routing: RoutingConfig,
    pub logging: LoggingConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProvidersConfig {
    pub openai: ProviderConfig,
    pub zhipu: ProviderConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProviderConfig {
    pub api_key: String,
    #[serde(default = "default_url")]
    pub base_url: String,
    #[serde(default = "default_model")]
    pub default_model: String,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_url() -> String {
    "".to_string()
}

fn default_model() -> String {
    "gpt-4o".to_string()
}

fn default_timeout() -> u64 {
    60
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RoutingConfig {
    #[serde(default = "default_provider")]
    pub default: String,
    pub model_mapping: Option<std::collections::HashMap<String, String>>,
}

fn default_provider() -> String {
    "openai".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

impl Config {
    pub fn load() -> Result<Self, crate::error::Error> {
        // Try to load from config.yaml first
        let config_path = std::path::Path::new("config.yaml");

        if config_path.exists() {
            let config_str = std::fs::read_to_string(config_path)?;
            let mut config: Config = serde_yaml::from_str(&config_str)?;
            config.load_env_vars();
            return Ok(config);
        }

        // Return default config if no config file exists
        Ok(Config::default())
    }

    fn load_env_vars(&mut self) {
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            self.providers.openai.api_key = api_key;
        }
        if let Ok(api_key) = std::env::var("ZHIPU_API_KEY") {
            self.providers.zhipu.api_key = api_key;
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            providers: ProvidersConfig {
                openai: ProviderConfig {
                    api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
                    base_url: "https://api.openai.com/v1".to_string(),
                    default_model: "gpt-4o".to_string(),
                    timeout: 120,
                },
                zhipu: ProviderConfig {
                    api_key: std::env::var("ZHIPU_API_KEY").unwrap_or_default(),
                    base_url: "https://open.bigmodel.cn/api/paas/v4".to_string(),
                    default_model: "glm-4".to_string(),
                    timeout: 60,
                },
            },
            routing: RoutingConfig {
                default: "openai".to_string(),
                model_mapping: Some(std::collections::HashMap::new()),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
        }
    }
}

impl ProvidersConfig {
    #[allow(dead_code)]
    pub fn get_provider(&self, name: &str) -> Option<&ProviderConfig> {
        match name {
            "openai" => Some(&self.openai),
            "zhipu" => Some(&self.zhipu),
            _ => None,
        }
    }
}
