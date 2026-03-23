use crate::config::LoggingConfig;
use std::fs::File;
use std::path::Path;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logging(config: &LoggingConfig) -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| default_env_filter(&config.level));
    let file = prepare_log_file(&config.file_path)?;
    let file_writer = move || file.try_clone().expect("log file clone should succeed");

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(file_writer),
        )
        .try_init()?;

    Ok(())
}

fn default_env_filter(level: &str) -> tracing_subscriber::EnvFilter {
    let directive = if level.contains('=') || level.contains(',') {
        level.to_string()
    } else {
        format!("openai_proxy={level},tower_http={level}")
    };

    tracing_subscriber::EnvFilter::new(directive)
}

fn prepare_log_file(path: &str) -> std::io::Result<File> {
    let path = Path::new(path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    File::create(path)
}

#[cfg(test)]
mod tests {
    use crate::config::LoggingConfig;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_prepare_log_file_truncates_existing_contents_and_creates_parent_dir() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("openai-proxy-log-test-{unique}"));
        let log_path = temp_dir.join("nested/server.log");
        fs::create_dir_all(log_path.parent().unwrap()).unwrap();
        fs::write(&log_path, "old log contents").unwrap();

        super::prepare_log_file(log_path.to_str().unwrap()).unwrap();

        let contents = fs::read_to_string(&log_path).unwrap();
        assert!(contents.is_empty(), "log file should be truncated on startup");

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn test_default_env_filter_uses_configured_level_for_targeted_modules() {
        let config = LoggingConfig {
            level: "info".to_string(),
            format: "json".to_string(),
            file_path: "logs/server.log".to_string(),
        };

        let filter = super::default_env_filter(&config.level);
        let filter_string = filter.to_string();

        assert!(filter_string.contains("openai_proxy=info"));
        assert!(filter_string.contains("tower_http=info"));
    }
}
