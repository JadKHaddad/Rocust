use std::sync::Arc;

use crate::traits::{HasTask, User};
use tokio::sync::Notify;

pub struct Test {
    pub count: i32,
    pub notify: Arc<Notify>,
}

pub enum Status {
    Running,
    Stopped,
    Finished,
}
impl Test {
    pub fn new(count: i32) -> Self {
        Test {
            count,
            notify: Arc::new(Notify::new()),
        }
    }

    // no need
    // pub async fn run<T>(&self) -> Status
    // where
    //     T: HasTask + User + Default + Send,
    // {
    //     tokio::select! {
    //         _ = self.run_users::<T>() => {
    //             println!("run_users finished");
    //             unreachable!()
    //         }
    //         _ = self.notify.notified() => {
    //             println!("notify finished");
    //             //this is the ctrl+c
    //             Status::Stopped
    //         }
    //         _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
    //             //this is the run time
    //             println!("sleep finished");
    //             Status::Finished
    //         }
    //     }
    //     // we still need to run the on_stop
    // }

    pub async fn run_users<T>(&self)
    where
        T: HasTask + User + Default + Send + 'static,
    {
        let mut handles = vec![];
        for _ in 0..self.count {
            let notify = self.notify.clone();
            let handle = tokio::spawn(async move {
                let mut user = T::default();
                user.inject_tasks();
                user.on_start();
                let tasks = user.get_tasks();
                loop {
                    // get a random task
                    // call it
                    let task = tasks.get(0).unwrap(); 
                    task.call(&mut user); // should be async so we can also select
                    // do some sleep or stop
                    tokio::select! {
                        _ = notify.notified() => {
                            break;
                        }
                        _ = tokio::time::sleep(std::time::Duration::from_millis(1000)) => {
                        }
                    }
                }
                
                user
            });
            handles.push(handle);
        }
        for handle in handles {
            let mut user = handle.await.unwrap();
            user.on_stop();
        }
        // run time is not defined yet
    }
}
