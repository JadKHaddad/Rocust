use async_trait::async_trait;
use reqwest::Client;
use rocust::{
    rocust_lib::{run, Context, Shared, Test, TestConfig, User},
    rocust_macros::has_task,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::{signal, sync::RwLock};

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

#[derive(Clone)]
struct GoogleUser {
    id: u64,
    client: Client,
    host: &'static str,
}

#[has_task(min_sleep = 10, max_sleep = 20, weight = 1)]
impl GoogleUser {
    #[task(priority = 40)]
    pub async fn index(&mut self, context: &Context) {
        let start = std::time::Instant::now();
        let res = self
            .client
            .get(format!("https://{}", self.host))
            .send()
            .await;
        let end = std::time::Instant::now();
        match res {
            Ok(res) => {
                if res.status().is_success() {
                    let duration = end.duration_since(start);
                    let duration = duration.as_secs_f64();
                    context.add_success(String::from("GET"), format!("{}/", self.host), duration);
                } else {
                    context.add_failure(String::from("GET"), format!("{}/", self.host));
                }
            }
            Err(_) => {
                context.add_error(
                    String::from("GET"),
                    format!("{}/", self.host),
                    String::from("error"),
                );
            }
        }
    }

    #[task(priority = 40)]
    pub async fn none_existing_path(&mut self, context: &Context) {
        let start = std::time::Instant::now();
        let res = self
            .client
            .get(format!("https://{}/none_existing_path", self.host))
            .send()
            .await;
        let end = std::time::Instant::now();
        match res {
            Ok(res) => {
                if res.status().is_success() {
                    let duration = end.duration_since(start);
                    let duration = duration.as_secs_f64();
                    context.add_success(
                        String::from("GET"),
                        format!("{}/none_existing_path", self.host),
                        duration,
                    );
                } else {
                    context.add_failure(
                        String::from("GET"),
                        format!("{}/none_existing_path", self.host),
                    );
                }
            }
            Err(_) => {
                context.add_error(
                    String::from("GET"),
                    format!("{}/none_existing_path", self.host),
                    String::from("error"),
                );
            }
        }
    }

    #[task(priority = 1)]
    async fn will_panic(&mut self, _context: &Context) {
        panic!("This task will panic");
    }

    #[task(priority = 1)]
    async fn suicide(&mut self, context: &Context) {
        context.stop();
    }
}

#[async_trait]
impl User for GoogleUser {
    type Shared = ();

    async fn new(_test_config: &TestConfig, context: &Context, _shared: Self::Shared) -> Self {
        let client = Client::new();
        GoogleUser {
            id: context.get_id(),
            client,
            host: "google.com",
        }
    }

    async fn on_start(&mut self, _: &Context) {
        println!("GoogleUser {} started", self.id);
    }

    async fn on_stop(&mut self, _: &Context) {
        println!("GoogleUser {} stopped", self.id);
    }
}

#[derive(Clone)]
struct FacebookUser {
    client: Client,
}

#[has_task(min_sleep = 10, max_sleep = 20, weight = 1)]
impl FacebookUser {
    #[task(priority = 10)]
    pub async fn index(&mut self, context: &Context) {
        let start = std::time::Instant::now();
        let res = self
            .client
            .get(String::from("https://facebook.com"))
            .send()
            .await;
        let end = std::time::Instant::now();
        match res {
            Ok(res) => {
                if res.status().is_success() {
                    let duration = end.duration_since(start);
                    let duration = duration.as_secs_f64();
                    context.add_success(
                        String::from("GET"),
                        String::from("facebook.com/"),
                        duration,
                    );
                } else {
                    context.add_failure(String::from("GET"), String::from("facebook.com/"));
                }
            }
            Err(_) => {
                context.add_error(
                    String::from("GET"),
                    String::from("facebook.com/"),
                    String::from("error"),
                );
            }
        }
    }

    #[task(priority = 1)]
    async fn suicide(&mut self, context: &Context) {
        context.stop();
    }
}

#[async_trait]
impl User for FacebookUser {
    type Shared = ();
    async fn new(_test_config: &TestConfig, _context: &Context, _shared: Self::Shared) -> Self {
        let client = Client::new();
        FacebookUser { client }
    }

    async fn on_start(&mut self, context: &Context) {
        println!("FacebookUser {} started", context.get_id());
    }

    async fn on_stop(&mut self, context: &Context) {
        println!("FacebookUser {} stopped", context.get_id());
    }
}

#[tokio::main]
async fn main() {
    // export RUSTFLAGS="--cfg tokio_unstable"
    // export ROCUST_LOG="debug"
    // $Env:RUSTFLAGS="--cfg tokio_unstable"
    // $Env:ROCUST_LOG="info"
    // console_subscriber::init();

    let test_config = TestConfig::default()
        .user_count(100)
        .users_per_sec(1)
        .runtime(6000)
        .update_interval_in_secs(2)
        .print_to_stdout(true)
        .log_to_stdout(true)
        .log_level(tracing::level_filters::LevelFilter::INFO)
        .log_file(String::from("results/log.log"))
        .current_results_file(String::from("results/current_results.csv"))
        .results_history_file(String::from("results/results_history.csv"))
        .summary_file(String::from("results/summary.yaml"))
        .prometheus_current_metrics_file(String::from("results/current_metrics.prom"))
        .prometheus_metrics_history_folder(String::from("results/metrics_history"))
        .server_address(SocketAddr::from(([127, 0, 0, 1], 3000)))
        .additional_args(vec![])
        .additional_arg(String::from("test"));
    // .stop_condition(|stop_condition_data| {
    //     if stop_condition_data
    //         .get_all_results()
    //         .get_aggrigated_results()
    //         .get_total_failed_requests()
    //         >= 200
    //     {
    //         return true;
    //     }
    //     false
    // });

    // or get test config from CLI. stop_condition will be ignored for now (will be implemented later)
    // cargo run -p dev -- --user-count 20 --users-per-sec 4 --runtime 60 --update-interval-in-secs 3 --log-level "debug" --log-file "results/log.log" --current-results-file "results/current_results.csv" --results-history-file "results/results_history.csv" --summary-file "results/summary.json" --server-address "127.0.0.1:8080" --additional-arg "arg1" --additional-arg "arg2"
    // let test_config = TestConfig::from_cli_args().expect("Failed to get test config from CLI args");

    let mut test = Test::new(test_config).await;
    let test_controller = test.create_test_controller();

    // stop test on ctrl+c
    tokio::spawn(async move {
        signal::ctrl_c().await.unwrap();
        test_controller.stop();
    });

    run!(test, GoogleUser, FacebookUser).await;

    // tokio::time::sleep(std::time::Duration::from_secs(60)).await;
}
