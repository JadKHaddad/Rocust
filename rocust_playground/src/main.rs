use async_trait::async_trait;
use rocust::rocust_lib::{
    test::spawn_coordinator::{SpawnCoordinator, UserSpawnController},
    traits::HasTask,
    Context, TestConfig, User,
};

#[allow(dead_code)]

struct MyUser {
    id: u64,
}

#[allow(clippy::all)]
impl MyUser {
    async fn suicide(&mut self, context: &Context) {
        context.stop().await;
    }

    fn blocking(mut self, context: Context) -> (Self, Context) {
        self.id = self.id + 1;
        let body = reqwest::blocking::get("https://www.rust-lang.org")
            .unwrap()
            .text()
            .unwrap();
        println!("{}", body);
        (self, context)
    }
}

#[allow(clippy::all)]
impl HasTask for MyUser {
    fn get_async_tasks() -> Vec<rocust::rocust_lib::tasks::AsyncTask<Self>> {
        let mut async_tasks = vec![];

        fn suicide<'a>(
            u: &'a mut MyUser,
            context: &'a rocust::rocust_lib::test::user::context::Context,
        ) -> ::core::pin::Pin<
            Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'a>,
        > {
            Box::pin(async move {
                u.suicide(context).await;
            })
        }

        async_tasks.push(rocust::rocust_lib::tasks::AsyncTask::new(
            1, "suicide", suicide,
        ));

        async_tasks
    }

    fn get_name() -> &'static str {
        "MyUser"
    }
}

#[allow(clippy::all)]
#[async_trait]
impl User for MyUser {
    type Shared = ();
    async fn new(_test_config: &TestConfig, _context: &Context, _shared: Self::Shared) -> Self {
        MyUser { id: 0 }
    }
}

#[tokio::main]
async fn main() {
    // let mut blocking_tasks = vec![];
    // fn blocking(
    //     u: MyUser,
    //     context: rocust::rocust_lib::test::user::context::Context,
    // ) -> ::core::pin::Pin<
    //     Box<
    //         dyn ::core::future::Future<
    //                 Output = Result<
    //                     (MyUser, rocust::rocust_lib::test::user::context::Context),
    //                     tokio::task::JoinError,
    //                 >,
    //             > + ::core::marker::Send,
    //     >,
    // > {
    //     Box::pin(async move { tokio::task::spawn_blocking(move || u.blocking(context)).await })
    // }

    // blocking_tasks.push(rocust::rocust_lib::tasks::BlockingTask::new(
    //     1, "blocking", blocking,
    // ));

    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "rocust=trace");
    }
    tracing_subscriber::fmt::init();

    let f_controller = UserSpawnController {
        user_name: String::from("FacebookUser"),
        count: 33,
        total_spawned: 0,
    };

    let g_controller = UserSpawnController {
        user_name: String::from("GoogleUser"),
        count: 20,
        total_spawned: 0,
    };

    let k_controller = UserSpawnController {
        user_name: String::from("KontentUser"),
        count: 10,
        total_spawned: 0,
    };

    let controllers = vec![f_controller, g_controller, k_controller];
    let mut coordinator = SpawnCoordinator {
        users_per_sec: 50,
        user_spawn_controllers: controllers,
    };

    coordinator.spawn().await;
}
