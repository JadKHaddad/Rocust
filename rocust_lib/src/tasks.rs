#[derive(Clone)]
pub struct Task<T> {
    priority: i32,
    func: fn(&mut T)->(),
}

impl<T> Task<T> {
    pub fn new(priority: i32, func: fn(&mut T)->()) -> Self {
        Task {
            priority,
            func,
        }
    }

    pub fn call(&self, user: &mut T) {
        (self.func)(user);
    }
}