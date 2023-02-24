use async_trait::async_trait;
use reqwest::Client;
use rocust::{
    rocust_lib::{
        data::Data,
        run,
        test::Test,
        test_config::TestConfig,
        traits::{Shared, User},
    },
    rocust_macros::has_task,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;

#[allow(dead_code)]
#[derive(Clone)]
struct MyShared {
    pub some_shared: Arc<RwLock<i32>>,
}

#[async_trait]
impl Shared for MyShared {
    async fn new() -> Self {
        MyShared {
            some_shared: Arc::new(RwLock::new(0)),
        }
    }
}

struct MyUser {
    id: u64,
    client: Client,
}

#[has_task(between = "(3, 5)", weight = 1)]
impl MyUser {
    #[task(priority = 20)]
    pub async fn index(&mut self, data: &Data) {
        let start = std::time::Instant::now();
        let res = self.client.get("https://google.com").send().await;
        let end = std::time::Instant::now();
        match res {
            Ok(res) => {
                if res.status().is_success() {
                    let duration = end.duration_since(start);
                    let duration = duration.as_secs_f64();
                    data.get_events_handler().add_success(
                        String::from("GET"),
                        String::from("/"),
                        duration,
                    );
                } else {
                    data.get_events_handler()
                        .add_failure(String::from("GET"), String::from("/"));
                }
            }
            Err(_) => {
                data.get_events_handler().add_error(
                    String::from("GET"),
                    String::from("/"),
                    String::from("error"),
                );
            }
        }
    }

    #[task(priority = 20)]
    pub async fn none_existing_path(&mut self, data: &Data) {
        let start = std::time::Instant::now();
        let res = self
            .client
            .get("https://google.com/none_existing_path")
            .send()
            .await;
        let end = std::time::Instant::now();
        match res {
            Ok(res) => {
                if res.status().is_success() {
                    let duration = end.duration_since(start);
                    let duration = duration.as_secs_f64();
                    data.get_events_handler().add_success(
                        String::from("GET"),
                        String::from("/none_existing_path"),
                        duration,
                    );
                } else {
                    data.get_events_handler()
                        .add_failure(String::from("GET"), String::from("/none_existing_path"));
                }
            }
            Err(_) => {
                data.get_events_handler().add_error(
                    String::from("GET"),
                    String::from("/none_existing_path"),
                    String::from("error"),
                );
            }
        }
    }

    #[task(priority = 1)]
    async fn will_panic(&mut self, _data: &Data) {
        panic!("This task will panic");
    }

    #[task(priority = 10)]
    async fn suicide(&mut self, data: &Data) {
        data.get_user_controller().stop();
    }
}

#[async_trait]
impl User for MyUser {
    type Shared = ();

    async fn new(_test_config: &TestConfig, data: &Data, _shared: Self::Shared) -> Self {
        let client = Client::new();
        MyUser {
            id: data.get_events_handler().get_id(),
            client,
        }
    }

    async fn on_start(&mut self, _: &Data) {
        println!("User {} started", self.id);
    }

    async fn on_stop(&mut self, _: &Data) {
        println!("User {} stopped", self.id);
    }
}

struct MyUser2 {}

#[has_task(between = "(3, 5)", weight = 2)]
impl MyUser2 {
    #[task(priority = 10)]
    async fn suicide(&mut self, data: &Data) {
        data.get_user_controller().stop();
    }
}

#[async_trait]
impl User for MyUser2 {
    type Shared = ();
    async fn new(_test_config: &TestConfig, _data: &Data, _shared: Self::Shared) -> Self {
        MyUser2 {}
    }
}

#[tokio::main]
async fn main() {
    // export RUSTFLAGS="--cfg tokio_unstable"
    // export ROCUST_LOG="debug"
    // $Env:RUSTFLAGS="--cfg tokio_unstable"
    // $Env:ROCUST_LOG="debug"
    // console_subscriber::init();

    let test_config = TestConfig::new(
        20,
        10,
        Some(10),
        2,
        true,
        Some(tracing::level_filters::LevelFilter::INFO),
        Some(String::from("results/current_results.csv")),
        Some(String::from("results/results_history.csv")),
        Some(SocketAddr::from(([127, 0, 0, 1], 3000))),
        // additional args, will be provided via CLI
        vec![],
        // stop condition: stop the test when total failures >= 30
        // stop condition will be checked at the end of each update phase (every {update_interval} seconds})
        Some(|stop_condition_data| {
            if stop_condition_data
                .get_all_results()
                .get_aggrigated_results()
                .get_total_failed_requests()
                >= &200
            {
                return true;
            }
            false
        }),
    );

    // or get test config from CLI. server_address, stop_condition and additional_args will be ignored for now (will be implemented later)
    // cargo run -p dev -- --user-count 20 --users-per-sec 4 --runtime 60 --update-interval-in-secs 3 --log-level "debug" --current-results-file "results/current_results.csv" --results-history-file "results/results_history.csv" --server-address "127.0.0.1:8080" --additional-arg "arg1" --additional-arg "arg2"
    //let test_config = TestConfig::from_cli_args().expect("Failed to get test config from CLI args");

    let test = Test::new(test_config).await;
    let test_controller = test.create_test_controller();

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(100)).await;
        test_controller.stop();
    });

    run!(test, MyUser, MyUser2).await;

    //tokio::time::sleep(std::time::Duration::from_secs(60)).await;
}
