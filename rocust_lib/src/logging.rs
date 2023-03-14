use thiserror::Error as ThisError;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, Layer,
};

#[derive(Debug, ThisError)]
pub enum LogLevelError {
    #[error("Invalid log level: {0}")]
    InvalidLogLevel(String),
}

pub fn parse_log_level(log_level: &str) -> Result<LevelFilter, LogLevelError> {
    let log_level = log_level.to_lowercase();
    match log_level.as_str() {
        "trace" => Ok(LevelFilter::TRACE),
        "debug" => Ok(LevelFilter::DEBUG),
        "info" => Ok(LevelFilter::INFO),
        "warn" => Ok(LevelFilter::WARN),
        "error" => Ok(LevelFilter::ERROR),
        "off" => Ok(LevelFilter::OFF),
        _ => Err(LogLevelError::InvalidLogLevel(log_level.to_string())),
    }
}

fn log_level_from_env() -> LevelFilter {
    let log_level = std::env::var("ROCUST_LOG").unwrap_or_else(|_| "off".to_string());
    parse_log_level(&log_level).unwrap_or(LevelFilter::OFF)
}

// TODO: this is a bit of a mess
// TODO: REWOOOOOOORK
pub fn setup_logging(
    log_level: Option<LevelFilter>,
    log_to_stdout: bool,
    log_file: Option<String>,
) -> Option<WorkerGuard> {
    let log_level = if let Some(log_level) = log_level {
        log_level
    } else {
        log_level_from_env()
    };

    if let Some(log_file) = log_file {
        // get parent directory
        match std::path::Path::new(&log_file).parent() {
            Some(parent_dir) => {
                match std::path::Path::new(&log_file).file_name() {
                    Some(file_name) => {
                        let file_appender = tracing_appender::rolling::never(parent_dir, file_name);
                        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

                        let file_appender_layer = fmt::layer()
                            .with_writer(non_blocking)
                            .with_ansi(false)
                            .with_filter(log_level);

                        if log_to_stdout {
                            tracing_subscriber::registry()
                                .with(file_appender_layer)
                                .with(
                                    fmt::layer()
                                        .with_writer(std::io::stdout)
                                        .with_filter(log_level),
                                )
                                .init();
                        } else {
                            tracing_subscriber::registry()
                                .with(file_appender_layer)
                                .init();
                        }
                        return Some(guard);
                    }
                    None => {
                        tracing::error!("Failed to get file name of log file");
                        return None;
                    }
                };
            }
            None => {
                tracing::error!("Failed to get parent directory of log file");
                return None;
            }
        };
    }
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(std::io::stdout)
                .with_filter(log_level),
        )
        .init();
    None
}
