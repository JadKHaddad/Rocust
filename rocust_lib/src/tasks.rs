pub struct Task<T> {
    priority: i32,
    func: fn(&T)->(),
}

impl<T> Task<T> {
    pub fn new(priority: i32, func: fn(&T)->()) -> Self {
        Task {
            priority,
            func,
        }
    }
}