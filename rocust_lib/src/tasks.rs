use std::future::Future;
use std::pin::Pin;

use crate::events::EventsHandler;
use crate::traits::Prioritised;

#[derive(Clone)]
pub struct AsyncTask<T>
where
    T: 'static,
{
    priority: u64,
    func: for<'a> fn(&'a mut T, &'a EventsHandler) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>,
}

impl<T> AsyncTask<T> {
    pub fn new(
        priority: u64,
        func: for<'a> fn(
            &'a mut T,
            &'a EventsHandler,
        ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>,
    ) -> Self {
        AsyncTask { priority, func }
    }

    pub fn get_priority(&self) -> u64 {
        self.priority
    }

    pub async fn call(&self, user: &mut T, handler: &EventsHandler) {
        (self.func)(user, handler).await;
    }
}

impl<T> Prioritised for AsyncTask<T> {
    fn get_priority(&self) -> u64 {
        self.priority
    }
}

#[derive(Clone)]
pub struct Task<T> {
    priority: i32,
    func: fn(&mut T) -> (),
}

impl<T> Task<T> {
    pub fn new(priority: i32, func: fn(&mut T) -> ()) -> Self {
        Task { priority, func }
    }

    pub fn get_priority(&self) -> i32 {
        self.priority
    }

    pub fn call(&self, user: &mut T) {
        (self.func)(user);
    }
}
