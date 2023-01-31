use rocust::rocust_lib;
use rocust::rocust_macros;

#[rocust_macros::user]
#[derive(Default, Clone)]
pub struct MyUser {
    a: i32,
    b: i32,
    //pub results_sener: rocust_lib::results::ResultsSender
}

#[rocust_macros::has_task]
impl MyUser {
    #[task(priority = 1)]
    pub async fn foo(&mut self) {
        self.a += 1;
        println!("foo: {}", self.a);
        rocust::rocust_lib::events::add_success(self, String::from("GET"), String::from("/"), 0.1);
    }

    #[task(priority = 3)]
    pub async fn bar(&mut self) {
        self.b += 1;
        println!("bar: {}", self.b);
        rocust::rocust_lib::events::add_success(self, String::from("GET"), String::from("/"), 0.1);
    }

    //#[task(priority = 3)]
    pub fn print(&self) {
        println!("a: {}, b: {}", self.a, self.b);
    }
}

impl rocust_lib::traits::User for MyUser {
    fn on_create(&mut self, id: u16) {
        println!("on_create: {}", id);
    }

    fn on_start(&mut self) {
        println!("on_start");
    }

    fn on_stop(&mut self) {
        println!("on_stop");
    }
}

#[tokio::main]
async fn main() {
    let test = rocust_lib::test::Test::new(3, None);
    let notify = test.notify.clone();

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        notify.notify_waiters();
    });

    test.run::<MyUser>().await;
}
