pub trait HasTask {
    fn add_succ(&mut self, dummy: i32);
    fn add_fail(&mut self, dummy: i32);
    fn get_async_tasks() -> Vec<crate::tasks::AsyncTask<Self>>
    where
        Self: Sized,
    {
        vec![]
    }
}

pub trait User {
    fn on_create(&mut self, _id: u16) {}
    fn on_start(&mut self) {}
    fn on_stop(&mut self) {}
}

pub trait Prioritised {
    fn get_priority(&self) -> i32;
}
