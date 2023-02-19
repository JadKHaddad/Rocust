use crate::test::TestConfig as TestTestConfig;
use clap::Parser;

/// ROCUST
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct TestConfig {
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

    /// Print results to stdout.
    #[arg(long)]
    print_to_stdout: bool,

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

    /// Stop the test when the stop condition is met. The stop condition will be checked at the end of each update phase (every {update_interval} seconds}).
    #[arg(long)]
    stop_condition: Option<String>,
}

impl TestConfig {
    pub fn new() -> Self {
        TestConfig::parse()
    }
}

impl Into<TestTestConfig> for TestConfig {
    //TODO: parse server_address
    //TODO: parse stop_condition
    //TODO: parse additional_args
    fn into(self) -> TestTestConfig {
        TestTestConfig::new(
            self.user_count,
            self.users_per_sec,
            self.runtime,
            self.update_interval_in_secs,
            self.print_to_stdout,
            self.current_results_file,
            self.results_history_file,
            None,
            vec![],
            None,
        )
    }
}
