use rocust::rocust_lib::traits::Shared;
use rocust::rocust_lib::{results::EventsHandler, run, test::Test, traits::User};
use rocust::rocust_macros::has_task;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
struct MyShared {
    pub some_shared: Arc<RwLock<i32>>,
}

impl rocust::rocust_lib::traits::Shared for MyShared {
    fn new() -> Self {
        MyShared {
            some_shared: Arc::new(RwLock::new(0)),
        }
    }
}

struct MyUser {
    a: i32,
    b: i32,
    id: u16,
    shared: MyShared,
}

#[has_task(between = "(3, 5)", weight = 4)]
impl MyUser {
    #[task(priority = 1)]
    pub async fn foo(&mut self, handler: &EventsHandler) {
        self.a += 1;
        println!("{}: foo: {}", self.id, self.a);
        handler.add_success(String::from("GET"), String::from("/foo"), 0.1);
    }

    #[task(priority = 3)]
    pub async fn bar(&mut self, handler: &EventsHandler) {
        self.b += 1;
        println!("{} bar: {}", self.id, self.b);
        handler.add_failure(String::from("GET"), String::from("/bar"));

        // count failures in shared state
        let mut shared = self.shared.some_shared.write().await;
        *shared += 1;
    }

    #[task(priority = 2)]
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
}

impl User for MyUser {
    type Shared = MyShared;

    fn new(id: u16, handler: &EventsHandler, shared: MyShared) -> Self {
        handler.add_success(String::from("CREATE"), String::from(""), 0.0);
        MyUser {
            a: 0,
            b: 0,
            id,
            shared,
        }
    }

    fn on_start(&mut self, _: &EventsHandler) {
        println!("on_start: {}", self.id);
    }

    fn on_stop(&mut self, _: &EventsHandler) {
        println!("on_stop: {}", self.id);
    }
}

#[tokio::main]
async fn main() {
    let test = Test::new(10, 10, None);
    let token = test.token.clone();

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        token.cancel();
    });

    run!(test, MyUser);
}
