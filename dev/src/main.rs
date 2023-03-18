use async_trait::async_trait;
use reqwest::Client;
use rocust::{
    rocust_lib::{
        futures::RocustFutures,
        run, Context, Shared, Test, TestConfig, User,
    },
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

struct GoogleUser {
    id: u64,
    client: Client,
    host: &'static str,
}

#[has_task(min_sleep = 1, max_sleep = 2, weight = 1)]
impl GoogleUser {
    #[task(priority = 10)]
    pub async fn blocking_index(&mut self, context: &Context) {
        println!("GoogleUser [{}] performing blocking", self.id);
        let host = self.host;
        let res = tokio::task::spawn_blocking(move || {
            reqwest::blocking::get(format!("https://{}", host))
        })
        .await
        .unwrap();
        match res {
            Ok(res) => {
                if res.status().is_success() {
                    context
                        .add_success(String::from("BLOCKING GET"), format!("{}/", self.host), 0.0)
                        .await;
                } else {
                    context
                        .add_failure(String::from("BLOCKING GET"), format!("{}/", self.host))
                        .await;
                }
            }
            Err(_) => {
                context
                    .add_error(
                        String::from("BLOCKING GET"),
                        format!("{}/", self.host),
                        String::from("error"),
                    )
                    .await;
            }
        }
    }

    #[task(priority = 10)]
    pub async fn index(&mut self, context: &Context) {
        let (res, elapsed) = self
            .client
            .get(format!("https://{}", self.host))
            .send()
            .timed()
            .delayed(std::time::Duration::from_secs(1))
            .await;

        println!(
            "GoogleUser fetched [https://{}] with delay of 1 second",
            self.host
        );

        match res {
            Ok(res) => {
                if res.status().is_success() {
                    let elapsed = elapsed.as_secs_f64();
                    context
                        .add_success(String::from("GET"), format!("{}/", self.host), elapsed)
                        .await;
                } else {
                    context
                        .add_failure(String::from("GET"), format!("{}/", self.host))
                        .await;
                }
            }
            Err(_) => {
                context
                    .add_error(
                        String::from("GET"),
                        format!("{}/", self.host),
                        String::from("error"),
                    )
                    .await;
            }
        }
    }

    #[task(priority = 10)]
    pub async fn none_existing_path(&mut self, context: &Context) {
        let (res, elapsed) = self
            .client
            .get(format!("https://{}/none_existing_path", self.host))
            .send()
            .timed()
            .await;

        match res {
            Ok(res) => {
                if res.status().is_success() {
                    let elapsed = elapsed.as_secs_f64();
                    context
                        .add_success(
                            String::from("GET"),
                            format!("{}/none_existing_path", self.host),
                            elapsed,
                        )
                        .await;
                } else {
                    context
                        .add_failure(
                            String::from("GET"),
                            format!("{}/none_existing_path", self.host),
                        )
                        .await;
                }
            }
            Err(_) => {
                context
                    .add_error(
                        String::from("GET"),
                        format!("{}/none_existing_path", self.host),
                        String::from("error"),
                    )
                    .await;
            }
        }
    }

    #[task(priority = 1)]
    async fn will_panic(&mut self, _context: &Context) {
        panic!("This task will panic");
    }

    #[task(priority = 1)]
    async fn suicide(&mut self, context: &Context) {
        context.stop().await;
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
        println!("GoogleUser [{}] started", self.id);
    }

    async fn on_stop(&mut self, _: &Context) {
        println!("GoogleUser [{}] stopped", self.id);
    }
}

struct FacebookUser {
    client: Client,
}

#[has_task(min_sleep = 1, max_sleep = 2, weight = 1)]
impl FacebookUser {
    #[task(priority = 10)]
    pub async fn index(&mut self, context: &Context) {
        let ((res, elapsed), polls) = self
            .client
            .get(String::from("https://facebook.com"))
            .send()
            .timed()
            .counted()
            .await;
        println!(
            "FacebookUser performed {} polls to fetch [https://facebook.com]",
            polls
        );
        match res {
            Ok(res) => {
                if res.status().is_success() {
                    let elapsed = elapsed.as_secs_f64();
                    context
                        .add_success(String::from("GET"), String::from("facebook.com/"), elapsed)
                        .await;
                } else {
                    context
                        .add_failure(String::from("GET"), String::from("facebook.com/"))
                        .await;
                }
            }
            Err(_) => {
                context
                    .add_error(
                        String::from("GET"),
                        String::from("facebook.com/"),
                        String::from("error"),
                    )
                    .await;
            }
        }
    }

    #[task(priority = 1)]
    async fn suicide(&mut self, context: &Context) {
        context.stop().await;
        unreachable!();
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
        println!("FacebookUser [{}] started", context.get_id());
    }

    async fn on_stop(&mut self, context: &Context) {
        println!("FacebookUser [{}] stopped", context.get_id());
    }
}

#[tokio::main]
async fn main() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "rocust=trace");
    }
    tracing_subscriber::fmt::init();

    let test_config = TestConfig::default()
        .user_count(10)
        .users_per_sec(2)
        .runtime(6000)
        .update_interval_in_secs(2)
        .print_to_stdout(true)
        .current_results_file(String::from("results/current_results.csv"))
        .results_history_file(String::from("results/results_history.csv"))
        .summary_file(String::from("results/summary.json"))
        .prometheus_current_metrics_file(String::from("results/current_metrics.prom"))
        .prometheus_metrics_history_folder(String::from("results/metrics_history"))
        .server_address(SocketAddr::from(([127, 0, 0, 1], 3000)))
        .precision(3)
        .additional_args(vec![])
        .additional_arg(String::from("test"));

    // or get test config from CLI.
    // cargo run -p dev -- --user-count 20 --users-per-sec 4 --runtime 60 --update-interval-in-secs 3 --current-results-file "results/current_results.csv" --results-history-file "results/results_history.csv" --summary-file "results/summary.json" --server-address "127.0.0.1:8080" --additional-arg "arg1" --additional-arg "arg2"
    // let test_config = TestConfig::from_cli_args().expect("Failed to get test config from CLI args");

    let mut test = Test::new(test_config).await;
    let test_controller = test.create_test_controller();

    // stop test on ctrl+c
    tokio::spawn(async move {
        signal::ctrl_c().await.unwrap();
        test_controller.stop();
    });

    run!(test, FacebookUser, GoogleUser).await;
}
