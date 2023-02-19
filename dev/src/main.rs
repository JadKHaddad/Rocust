use async_trait::async_trait;
use reqwest::Client;
use rocust::{
    rocust_lib::{
        data::Data,
        events::EventsHandler,
        run,
        test::Test,
        test_config::TestConfig,
        traits::{Shared, User},
    },
    rocust_macros::has_task,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

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

#[has_task(between = "(3, 5)", weight = 1, name = "GoogleTester")]
impl MyUser {
    #[task(priority = 5)]
    pub async fn index(&mut self, data: &Arc<Data>) {
        let start = std::time::Instant::now();
        let res = self.client.get("https://google.com").send().await;
        let end = std::time::Instant::now();
        match res {
            Ok(res) => {
                if res.status().is_success() {
                    let duration = end.duration_since(start);
                    let duration = duration.as_secs_f64();
                    data.events_handler.add_success(
                        String::from("GET"),
                        String::from("/"),
                        duration,
                    );
                } else {
                    data.events_handler
                        .add_failure(String::from("GET"), String::from("/"));
                }
            }
            Err(_) => {
                data.events_handler.add_error(
                    String::from("GET"),
                    String::from("/"),
                    String::from("error"),
                );
            }
        }
    }

    #[task(priority = 5)]
    pub async fn none_existing_path(&mut self, data: &Arc<Data>) {
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
                    data.events_handler.add_success(
                        String::from("GET"),
                        String::from("/none_existing_path"),
                        duration,
                    );
                } else {
                    data.events_handler
                        .add_failure(String::from("GET"), String::from("/none_existing_path"));
                }
            }
            Err(_) => {
                data.events_handler.add_error(
                    String::from("GET"),
                    String::from("/none_existing_path"),
                    String::from("error"),
                );
            }
        }
    }

    #[task(priority = 1)]
    async fn will_panic(&mut self, _data: &Arc<Data>) {
        panic!("This task will panic");
    }
}

#[async_trait]
impl User for MyUser {
    type Shared = ();

    async fn new(id: u64, _data: &Arc<Data>, _shared: Self::Shared) -> Self {
        let client = Client::new();
        MyUser { id, client }
    }

    async fn on_start(&mut self, _: &Arc<Data>) {
        println!("User {} started", self.id);
    }

    async fn on_stop(&mut self, _: &Arc<Data>) {
        println!("User {} stopped", self.id);
    }
}

#[tokio::main]
async fn main() {
    // export RUSTFLAGS="--cfg tokio_unstable"
    // export RUST_LOG="debug"
    // $Env:RUSTFLAGS="--cfg tokio_unstable"
    // $Env:RUST_LOG="debug"
    // console_subscriber::init();

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let test_config = TestConfig::new(
        50,
        4,
        Some(60),
        2,
        true,
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
                >= &30
            {
                return true;
            }
            false
        }),
    );

    // or get test config from CLI. server_address, stop_condition and additional_args will be ignored for now (will be implemented later)
    // let test_config = TestConfig::from_cli_args();

    let test = Test::new(test_config).await;
    let test_controller = test.create_test_controller();

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
        test_controller.stop();
    });

    run!(test, MyUser).await;

    //tokio::time::sleep(std::time::Duration::from_secs(60)).await;
}
