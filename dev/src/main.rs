use async_trait::async_trait;
use rocust::{
    rocust_lib::{
        events::EventsHandler,
        run,
        test::{Test, TestConfig, TestController},
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
    a: i32,
    b: i32,
    id: u64,
    shared: MyShared,
}

#[has_task(between = "(3, 5)", weight = 4, name = "RoCustUnstableUser")]
impl MyUser {
    #[task(priority = 5)]
    pub async fn foo(&mut self, handler: &EventsHandler) {
        self.a += 1;
        println!("{}: foo: {}", self.id, self.a);
        handler.add_success(String::from("GET"), String::from("/foo"), 0.1);
    }

    #[task(priority = 6)]
    pub async fn bar(&mut self, handler: &EventsHandler) {
        self.b += 1;
        println!("{} bar: {}", self.id, self.b);
        handler.add_failure(String::from("GET"), String::from("/bar"));

        // count failures in shared state
        let mut shared = self.shared.some_shared.write().await;
        *shared += 1;
    }

    #[task(priority = 9)]
    pub async fn baz(&mut self, handler: &EventsHandler) {
        println!("{} baz: {}", self.id, self.a + self.b);
        handler.add_error(
            String::from("GET"),
            String::from("/baz"),
            String::from("error"),
        );

        // print shared state maybe?
        let shared = self.shared.some_shared.read().await;
        println!("shared: {}", *shared);
    }

    #[task(priority = 1)]
    pub async fn panic(&mut self, _handler: &EventsHandler) {
        panic!("panic");
    }
}

#[async_trait]
impl User for MyUser {
    type Shared = MyShared;

    async fn new(
        id: u64,
        _test_config: TestConfig,
        _test_controller: TestController,
        handler: &EventsHandler,
        shared: Self::Shared,
    ) -> Self {
        println!("MyUser Created!");
        handler.add_success(String::from("CREATE"), String::from(""), 0.0);
        MyUser {
            a: 0,
            b: 0,
            id,
            shared,
        }
    }

    async fn on_start(&mut self, _: &EventsHandler) {
        println!("on_start: {}", self.id);
    }

    async fn on_stop(&mut self, _: &EventsHandler) {
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
        Some(String::from("results/current_results.csv")),
        Some(String::from("results/results_history.csv")),
        Some(SocketAddr::from(([127, 0, 0, 1], 3000))),
    );
    let test = Test::new(test_config).await;
    let test_controller = test.get_controller();

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
        test_controller.stop();
    });

    run!(test, MyUser).await;

    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
}
