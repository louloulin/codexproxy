use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("配置错误: {0}")]
    Config(String),

    #[error("请求错误: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Provider 错误: {0}")]
    Provider(String),

    #[error("转换错误: {0}")]
    Transform(String),

    #[error("验证错误: {0}")]
    Validation(String),

    #[error("流式错误: {0}")]
    Streaming(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML 错误: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
