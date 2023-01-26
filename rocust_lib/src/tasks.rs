use std::future::Future;
use std::pin::Pin;

#[derive(Clone)]
pub struct Task<T> {
    priority: i32,
    func: fn(&mut T) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>,
}

impl<T> Task<T> {
    pub fn new(
        priority: i32,
        func: fn(&mut T) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>,
    ) -> Self {
        Task { priority, func }
    }

    pub fn get_priority(&self) -> i32 {
        self.priority
    }

    pub async fn call(&self, user: &mut T) {
        (self.func)(user).await;
    }
}
