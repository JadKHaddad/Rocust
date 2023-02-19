use async_trait::async_trait;
use rocust::{
    rocust_lib::{
        data::Data,
        events::EventsHandler,
        run,
        test::{Test, TestConfig},
        traits::{Shared, User},
    },
    rocust_macros::has_task,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

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

    shared: MyShared,
}

#[has_task(between = "(3, 5)", weight = 1, name = "RoCustUnstableUser")]
impl MyUser {
    #[task(priority = 5)]
    pub async fn foo(&mut self, data: &Arc<Data>) {

        data.events_handler
            .add_success(String::from("GET"), String::from("/foo"), 0.1);
    }

    #[task(priority = 6)]
    pub async fn bar(&mut self, data: &Arc<Data>) {

        data.events_handler
            .add_failure(String::from("GET"), String::from("/bar"));

    }

    #[task(priority = 9)]
    pub async fn baz(&mut self, data: &Arc<Data>) {

        data.events_handler.add_error(
            String::from("GET"),
            String::from("/baz"),
            String::from("error"),
        );

    }

    #[task(priority = 1)]
    pub async fn panic(&mut self, _data: &Arc<Data>) {
        panic!("panic");
    }
}

#[async_trait]
impl User for MyUser {
    type Shared = MyShared;

    async fn new(id: u64, data: &Arc<Data>, shared: Self::Shared) -> Self {

        MyUser { id, shared}
    }

    async fn on_start(&mut self, _: &Arc<Data>) {
        println!("on_start: {}", self.id);
    }

    async fn on_stop(&mut self, _: &Arc<Data>) {
        println!("on_stop: {}", self.id);
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
        vec![],
        Some(|stop_condition_data| {
            if stop_condition_data
                .get_all_results()
                .get_aggrigated_results()
                .get_total_requests()
                >= &10
            {
                return true;
            }
            false
        }),
    );
    let test = Test::new(test_config).await;
    let test_controller = test.create_test_controller();

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
        test_controller.stop();
    });

    run!(test, MyUser).await;

    //tokio::time::sleep(std::time::Duration::from_secs(60)).await;
}
