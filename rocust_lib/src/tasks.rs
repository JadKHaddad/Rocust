use crate::{test::user::context::Context, traits::Prioritised};
use std::{future::Future, pin::Pin};

type AsyncTaskFunctionSig<T> =
    for<'a> fn(&'a mut T, &'a Context) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

type BlockingTaskResult<T> = Result<(T, Context), tokio::task::JoinError>;
type BlockingTaskFunctionSig<T> =
    fn(T, Context) -> Pin<Box<dyn Future<Output = BlockingTaskResult<T>> + Send>>;

#[derive(Clone)]
pub struct AsyncTask<T>
where
    T: 'static,
{
    pub(crate) priority: u64,
    pub(crate) name: &'static str,
    pub(crate) func: AsyncTaskFunctionSig<T>,
}

impl<T> AsyncTask<T> {
    pub fn new(priority: u64, name: &'static str, func: AsyncTaskFunctionSig<T>) -> Self {
        AsyncTask {
            priority,
            name,
            func,
        }
    }

    pub fn get_priority(&self) -> u64 {
        self.priority
    }

    pub async fn call(&self, user: &mut T, context: &Context) {
        (self.func)(user, context).await;
    }
}

#[derive(Clone)]
pub struct BlockingTask<T> {
    pub(crate) priority: u64,
    pub(crate) name: &'static str,
    pub(crate) func: BlockingTaskFunctionSig<T>,
}

impl<T> BlockingTask<T> {
    pub fn new(priority: u64, name: &'static str, func: BlockingTaskFunctionSig<T>) -> Self {
        BlockingTask {
            priority,
            name,
            func,
        }
    }

    pub fn get_priority(&self) -> u64 {
        self.priority
    }

    pub async fn call(&self, user: T, context: Context) -> BlockingTaskResult<T> {
        (self.func)(user, context).await
    }
}

pub(crate) struct EventsTaskInfo {
    pub(crate) name: &'static str,
}

impl<T> Prioritised for AsyncTask<T> {
    fn get_priority(&self) -> u64 {
        self.priority
    }
}

impl<T> Prioritised for BlockingTask<T> {
    fn get_priority(&self) -> u64 {
        self.priority
    }
}
