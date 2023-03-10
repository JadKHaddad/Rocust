use async_trait::async_trait;
use rocust::rocust_lib::{traits::HasTask, Context, TestConfig, User};

#[allow(dead_code)]

struct MyUser<'a, 'b> {
    name: &'a str,
    street: &'b str,
}

impl<'a, 'b> MyUser<'a, 'b> {
    async fn suicide(&mut self, context: &Context) {
        context.stop();
    }
}

impl<'a, 'b> HasTask for MyUser<'static, 'static> {
    fn get_async_tasks() -> Vec<rocust::rocust_lib::tasks::AsyncTask<Self>> {
        let mut async_tasks = vec![];
        fn func<'a>(
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
            1,
            String::from("name"),
            func,
        ));
        async_tasks
    }
}

#[async_trait]
impl<'a, 'b> User for MyUser<'static, 'static> {
    type Shared = ();
    async fn new(_test_config: &TestConfig, _context: &Context, _shared: Self::Shared) -> Self {
        MyUser {
            name: "name",
            street: "street",
        }
    }
}

#[tokio::main]
async fn main() {}
