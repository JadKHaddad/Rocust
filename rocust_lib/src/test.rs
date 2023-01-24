use crate::traits::{HasTask, User};

pub struct Test {
    pub count: i32,
}

impl Test {
    pub async fn run<T>(&self)
    where
        T: HasTask + User + Default + Send,
    {
        let mut handles = vec![];
        for _ in 0..self.count {
            let handle = tokio::spawn(async move {
                let mut user = T::default();
                user.inject_tasks();
                user.on_start();
                let tasks = user.get_tasks();
                loop {
                    // get a random task
                    // call it
                    let task = tasks.get(0).unwrap();
                    task.call(&mut user);
                    // do some sleep
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                }
                //user.on_stop();
            });
            handles.push(handle);
        }
        for handle in handles {
            handle.await.unwrap();
        }
    }
}
