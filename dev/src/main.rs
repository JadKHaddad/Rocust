use rocust::rocust_lib;
use rocust::rocust_macros;

#[rocust_macros::user]
#[derive(Default, Clone)]
pub struct MyUser {
    a: i32,
    b: i32,
    //pub results: rocust_lib::results::Results
    //pub async_tasks: Vec<rocust_lib::tasks::AsyncTask<Self>>
    //pub tasks: Vec<rocust_lib::tasks::Task<Self>>
}

#[rocust_macros::has_task]
impl MyUser {
    #[task(priority = 1)]
    pub async fn foo(&mut self) {
        self.a += 1;
        println!("foo: {}", self.a);
    }

    #[task(priority = 3)]
    pub async fn bar(&mut self) {
        self.b += 1;
        println!("bar: {}", self.b);
    }

    #[task(priority = 3)]
    pub fn print(&self) {
        println!("a: {}, b: {}", self.a, self.b);
    }
}

impl rocust_lib::traits::User for MyUser {
    fn on_start(&mut self) {
        println!("on_start");
    }

    fn on_stop(&mut self) {
        println!("on_stop");
    }
}
#[tokio::main]
async fn main() {
    let test = rocust_lib::test::Test::new(3);
    let notify = test.notify.clone();

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        notify.notify_waiters();
    });

    //will panice because user has not tasks
    test.run::<MyUser>().await;
}
