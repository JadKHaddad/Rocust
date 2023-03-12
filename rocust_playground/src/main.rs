use async_trait::async_trait;
use rocust::rocust_lib::{traits::HasTask, Context, TestConfig, User};

#[allow(dead_code)]

struct MyUser {}

#[allow(clippy::all)]
impl MyUser {
    async fn suicide(&mut self, context: &Context) {
        context.stop();
    }

    fn blocking(mut self, context: Context) -> (Self, Context) {
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

        fn blocking(
            u: MyUser,
            context: rocust::rocust_lib::test::user::context::Context,
        ) -> ::core::pin::Pin<
            Box<
                dyn ::core::future::Future<
                        Output = Result<
                            (MyUser, rocust::rocust_lib::test::user::context::Context),
                            tokio::task::JoinError,
                        >,
                    > + ::core::marker::Send,
            >,
        > {
            Box::pin(async move { tokio::task::spawn_blocking(move || u.blocking(context)).await })
        }

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
        MyUser {}
    }
}

#[tokio::main]
async fn main() {
    let mut user = MyUser {};
    let tasks = MyUser::get_async_tasks();
    for task in tasks {}
}
