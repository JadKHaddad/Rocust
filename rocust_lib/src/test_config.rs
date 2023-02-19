use crate::data::StopConditionData;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone)]
pub struct TestConfig {
    pub user_count: u64,
    pub users_per_sec: u64,
    pub runtime: Option<u64>,
    pub update_interval_in_secs: u64,
    pub print_to_stdout: bool,
    pub current_results_file: Option<String>,
    pub results_history_file: Option<String>,
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
        current_results_file: Option<String>,
        results_history_file: Option<String>,
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
            current_results_file,
            results_history_file,
            server_address,
            additional_args,
            stop_condition,
        }
    }

    pub fn from_cli_args() -> Self {
        let external_test_config = ExternalTestConfig::parse();
        TestConfig::from(external_test_config)
    }

    pub fn from_json_string(json_string: &str) -> Self {
        todo!()
    }

    pub async fn from_json_file(json_file_path: &str) -> Self {
        todo!()
    }

    pub fn from_yaml_string(yaml_string: &str) -> Self {
        todo!()
    }

    pub async fn from_yaml_file(yaml_file_path: &str) -> Self {
        todo!()
    }

    pub fn from_toml_string(toml_string: &str) -> Self {
        todo!()
    }

    pub async fn from_toml_file(toml_file_path: &str) -> Self {
        todo!()
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

impl From<ExternalTestConfig> for TestConfig {
    //TODO: parse server_address
    //TODO: parse stop_condition
    //TODO: parse additional_args
    fn from(external_test_config: ExternalTestConfig) -> Self {
        Self {
            user_count: external_test_config.user_count,
            users_per_sec: external_test_config.users_per_sec,
            runtime: external_test_config.runtime,
            update_interval_in_secs: external_test_config.update_interval_in_secs,
            print_to_stdout: !external_test_config.no_print_to_stdout,
            current_results_file: external_test_config.current_results_file,
            results_history_file: external_test_config.results_history_file,
            server_address: None,
            additional_args: vec![],
            stop_condition: None,
        }
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
    #[arg(long, default_value_t = 1)]
    update_interval_in_secs: u64,

    /// Do not print results to stdout.
    #[arg(long)]
    no_print_to_stdout: bool,

    /// Path to the file where the current results should be written to. If not set, the results will not be written to a file.
    #[arg(long, default_value = None)]
    current_results_file: Option<String>,

    /// Path to the file where the results history should be written to. If not set, the results will not be written to a file.
    #[arg(long, default_value = None)]
    results_history_file: Option<String>,

    /// Address for the server to listen on. If not set, the server will not be started.
    #[arg(long, default_value = None)]
    server_address: Option<String>,

    /// Additional args, will be passed to the users.
    #[arg(long)]
    additional_args: Vec<String>,

    /// Stop the test when the stop condition is met. The stop condition will be checked at the end of each update phase (every {update_interval} seconds).
    #[arg(long)]
    stop_condition: Option<String>,
}
