use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Clone)]
pub struct Task<T> {
    priority: i32,
    func: fn(&mut T) -> Pin<Arc<Box<dyn Future<Output = ()>>>>,
}

impl<T> Task<T> {
    pub fn new(priority: i32, func: fn(&mut T) -> Pin<Arc<Box<dyn Future<Output = ()>>>>) -> Self {
        Task { priority, func }
    }

    pub fn get_priority(&self) -> i32 {
        self.priority
    }

    pub async fn call(&self, user: &mut T) {
        (self.func)(user);
    }
}
