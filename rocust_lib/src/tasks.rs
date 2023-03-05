use crate::{test::user::context::Context, traits::Prioritised};
use std::{future::Future, pin::Pin};

#[derive(Clone)]
pub struct AsyncTask<T>
where
    T: 'static,
{
    priority: u64,
    func: for<'a> fn(&'a mut T, &'a Context) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>,
}

impl<T> AsyncTask<T> {
    pub fn new(
        priority: u64,
        func: for<'a> fn(&'a mut T, &'a Context) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>,
    ) -> Self {
        AsyncTask { priority, func }
    }

    pub fn get_priority(&self) -> u64 {
        self.priority
    }

    pub async fn call(&self, user: &mut T, data: &Context) {
        (self.func)(user, data).await;
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
