use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("请求错误: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Provider 错误: {0}")]
    Provider(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML 错误: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Bad Request: {0}")]
    BadRequest(String),
}
