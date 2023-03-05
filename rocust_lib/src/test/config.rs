use crate::{
    fs::reader::{CreateError, ReadError, Reader},
    logging::{parse_log_level, LogLevelError},
    test::controller::StopConditionData,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{self, Error as SerdeJsonError};
use serde_yaml::{self, Error as SerdeYamlError};
use std::net::{AddrParseError, SocketAddr};
use thiserror::Error as ThisError;
use toml::de::Error as TomlDeError;
use tracing::level_filters::LevelFilter;

#[derive(Clone)]
pub struct TestConfig {
    pub user_count: u64,
    pub users_per_sec: u64,
    pub runtime: Option<u64>,
    pub update_interval_in_secs: u64,
    pub print_to_stdout: bool,
    pub log_to_stdout: bool,
    pub log_level: Option<LevelFilter>,
    pub log_file: Option<String>,
    pub current_results_file: Option<String>,
    pub results_history_file: Option<String>,
    pub summary_file: Option<String>,
    pub server_address: Option<SocketAddr>,
    pub additional_args: Vec<String>,
    // a stop condiction will be checked at the end of every update interval and will stop the test if it returns true
    pub stop_condition: Option<fn(StopConditionData) -> bool>,
}

impl TestConfig {
    pub fn new(
        user_count: u64,
        users_per_sec: u64,
        runtime: Option<u64>,
        update_interval_in_secs: u64,
        print_to_stdout: bool,
        log_to_stdout: bool,
        log_level: Option<LevelFilter>,
        log_file: Option<String>,
        current_results_file: Option<String>,
        results_history_file: Option<String>,
        summary_file: Option<String>,
        server_address: Option<SocketAddr>,
        additional_args: Vec<String>,
        stop_condition: Option<fn(StopConditionData) -> bool>,
    ) -> Self {
        Self {
            user_count,
            users_per_sec,
            runtime,
            update_interval_in_secs,
            print_to_stdout,
            log_to_stdout,
            log_level,
            log_file,
            current_results_file,
            results_history_file,
            summary_file,
            server_address,
            additional_args,
            stop_condition,
        }
    }

    pub fn from_cli_args() -> Result<Self, FromExternalTestConfigError> {
        let external_test_config = ExternalTestConfig::parse();
        Ok(TestConfig::try_from(external_test_config)?)
    }

    pub fn from_json_string(json_string: &str) -> Result<Self, FromJsonError> {
        let external_test_config = serde_json::from_str::<ExternalTestConfig>(json_string)?;
        Ok(TestConfig::try_from(external_test_config)?)
    }

    pub async fn from_json_file(json_file_path: &str) -> Result<Self, FromJsonFileError> {
        let reader = Reader::from_str(json_file_path).await?;
        let json_string = reader.read_all_to_string().await?;
        Ok(TestConfig::from_json_string(&json_string)?)
    }

    pub fn from_yaml_string(yaml_string: &str) -> Result<Self, FromYamlError> {
        let external_test_config = serde_yaml::from_str::<ExternalTestConfig>(yaml_string)?;
        Ok(TestConfig::try_from(external_test_config)?)
    }

    pub async fn from_yaml_file(yaml_file_path: &str) -> Result<Self, FromYamlFileError> {
        let reader = Reader::from_str(yaml_file_path).await?;
        let yaml_string = reader.read_all_to_string().await?;
        Ok(TestConfig::from_yaml_string(&yaml_string)?)
    }

    pub fn from_toml_string(toml_string: &str) -> Result<Self, FromTomlError> {
        let external_test_config = toml::from_str::<ExternalTestConfig>(toml_string)?;
        Ok(TestConfig::try_from(external_test_config)?)
    }

    pub async fn from_toml_file(toml_file_path: &str) -> Result<Self, FromTomlFileError> {
        let reader = Reader::from_str(toml_file_path).await?;
        let toml_string = reader.read_all_to_string().await?;
        Ok(TestConfig::from_toml_string(&toml_string)?)
    }

    pub async fn from_file(file_path: &str) -> Self {
        //TODO: get file extension and call the corresponding function
        //TODO: if file extension is not supported, try to parse it as json, yaml or toml or return an error
        todo!()
    }

    pub fn from_env() -> Self {
        todo!()
    }

    pub async fn from_console() -> Self {
        todo!()
    }
}

impl TryFrom<ExternalTestConfig> for TestConfig {
    type Error = FromExternalTestConfigError;

    fn try_from(external_test_config: ExternalTestConfig) -> Result<Self, Self::Error> {
        let log_level = if let Some(log_level) = external_test_config.log_level {
            Some(parse_log_level(&log_level)?)
        } else {
            None
        };
        let server_address: Option<SocketAddr> =
            if let Some(server_address) = external_test_config.server_address {
                Some(server_address.parse()?)
            } else {
                None
            };
        Ok(Self {
            user_count: external_test_config.user_count,
            users_per_sec: external_test_config.users_per_sec,
            runtime: external_test_config.runtime,
            update_interval_in_secs: external_test_config.update_interval_in_secs,
            print_to_stdout: !external_test_config.no_print_to_stdout,
            log_to_stdout: !external_test_config.no_log_to_stdout,
            log_level,
            log_file: external_test_config.log_file,
            current_results_file: external_test_config.current_results_file,
            results_history_file: external_test_config.results_history_file,
            summary_file: external_test_config.summary_file,
            server_address,
            additional_args: external_test_config.additional_arg,
            stop_condition: None,
        })
    }
}

/// ROCUST
#[derive(Parser, Debug, Clone, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
struct ExternalTestConfig {
    /// Total count of users to spawn concurrently.
    #[arg(long, default_value_t = 1)]
    user_count: u64,

    /// Count of users to spawn per second.
    #[arg(long, default_value_t = 1)]
    users_per_sec: u64,

    /// Runtime in seconds. If not set, the program will run forever.
    #[arg(long, default_value = None)]
    runtime: Option<u64>,

    /// Update interval in seconds. How often should the program update it's internal state.
    #[arg(long, default_value_t = 2)]
    update_interval_in_secs: u64,

    /// Do not print results to stdout.
    #[arg(long)]
    no_print_to_stdout: bool,

    /// Do not log to stdout.
    #[arg(long)]
    no_log_to_stdout: bool,

    /// Log level. Possible values: trace, debug, info, warn, error, off. If not set, will fall back to ROCUST_LOG.
    #[arg(long, default_value = None)]
    log_level: Option<String>,

    /// Path to the file where the log should be written to. If not set, the log will not be written to a file.
    #[arg(long, default_value = None)]
    log_file: Option<String>,

    /// Path to the file where the current results should be written to. If not set, the results will not be written to a file.
    #[arg(long, default_value = None)]
    current_results_file: Option<String>,

    /// Path to the file where the results history should be written to. If not set, the results will not be written to a file.
    #[arg(long, default_value = None)]
    results_history_file: Option<String>,

    /// Path to the file where the summary should be written to. If not set, the summary will not be written to a file.
    #[arg(long, default_value = None)]
    summary_file: Option<String>,

    /// Address for the server to listen on. If not set, the server will not be started.
    #[arg(long, default_value = None)]
    server_address: Option<String>,

    /// Additional args, will be passed to the users.
    #[arg(long, action = clap::ArgAction::Append)]
    additional_arg: Vec<String>,

    /// [Not supported yet] Stop the test when the stop condition is met. The stop condition will be checked at the end of each update phase (every {update_interval} seconds).
    #[arg(long)]
    stop_condition: Option<String>,
}

#[derive(Debug, ThisError)]
pub enum FromExternalTestConfigError {
    #[error("Error while parsing server address: {0}")]
    ServerAddressParseError(#[from] AddrParseError),

    #[error("Error while parsing log level: {0}")]
    LogLevelError(#[from] LogLevelError),
}

#[derive(Debug, ThisError)]
pub enum FromJsonError {
    #[error("Error while parsing json: {0}")]
    SerdeJsonError(#[from] SerdeJsonError),
    #[error("Error while converting to TestConfig: {0}")]
    ConversionError(#[from] FromExternalTestConfigError),
}

#[derive(Debug, ThisError)]
pub enum FromJsonFileError {
    #[error("Error while parsing json file: {0}")]
    FromJson(#[from] FromJsonError),
    #[error("Error while reading json file: {0}")]
    ReadError(#[from] ReadError),
    #[error("Error while creating reader: {0}")]
    CreateError(#[from] CreateError),
}

#[derive(Debug, ThisError)]
pub enum FromYamlError {
    #[error("Error while parsing yaml: {0}")]
    SerdeYamlError(#[from] SerdeYamlError),
    #[error("Error while converting to TestConfig: {0}")]
    ConversionError(#[from] FromExternalTestConfigError),
}

#[derive(Debug, ThisError)]
pub enum FromYamlFileError {
    #[error("Error while parsing yaml file: {0}")]
    FromYaml(#[from] FromYamlError),
    #[error("Error while reading yaml file: {0}")]
    ReadError(#[from] ReadError),
    #[error("Error while creating reader: {0}")]
    CreateError(#[from] CreateError),
}

#[derive(Debug, ThisError)]
pub enum FromTomlError {
    #[error("Error while parsing toml: {0}")]
    SerdeTomlError(#[from] TomlDeError),
    #[error("Error while converting to TestConfig: {0}")]
    ConversionError(#[from] FromExternalTestConfigError),
}

#[derive(Debug, ThisError)]
pub enum FromTomlFileError {
    #[error("Error while parsing toml file: {0}")]
    FromToml(#[from] FromTomlError),
    #[error("Error while reading toml file: {0}")]
    ReadError(#[from] ReadError),
    #[error("Error while creating reader: {0}")]
    CreateError(#[from] CreateError),
}

impl std::fmt::Display for TestConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "user_count: {}, users_per_sec: {}, runtime: {:?}, update_interval_in_secs: {}, print_to_stdout: {}, current_results_file: {:?}, results_history_file: {:?}, server_address: {:?}, additional_args: {:?}",
            self.user_count, self.users_per_sec, self.runtime, self.update_interval_in_secs, self.print_to_stdout, self.current_results_file, self.results_history_file, self.server_address, self.additional_args
        )
    }
}
