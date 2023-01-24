pub trait HasTask {
    fn add_succ(&mut self, dummy: i32);
    fn add_fail(&mut self, dummy: i32);
    fn inject_tasks(&mut self) {}
    fn get_tasks(&self) -> Vec<crate::tasks::Task<Self>>
    where
        Self: Sized,
    {
        vec![]
    }
}

pub trait User {
    fn on_start(&mut self) {}
    fn on_stop(&mut self) {}
}
