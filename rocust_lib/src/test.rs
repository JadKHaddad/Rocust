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

    pub async fn run<T>(&self)
    where
        T: HasTask + User + Default + Send,
    {
        let mut handles = vec![];
        for _ in 0..self.count {
            //control the spawn rate
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

                    let task_call_and_sleep = async {
                        task.call(&mut user).await; // should be async
                        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    };

                    // do some sleep or stop
                    tokio::select! {
                        _ = notify.notified() => {
                            break;
                        }
                        _ = task_call_and_sleep => {
                        }
                    }
                }
                user.on_stop();
            });
            handles.push(handle);
        }
        //start a timer in another task
        let notify = self.notify.clone();
        let timer = tokio::spawn(async move {
            // this is the run time
            tokio::select! {
                // this is the ctrl+c
                _ = notify.notified() => {
                    println!("received signal");
                }
                // this is the run time
                _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
                    println!("timer finished");
                    notify.notify_waiters();
                }
            }
        });

        for handle in handles {
            handle.await.unwrap();
        }
        println!("all users finished");

        timer.await.unwrap();
        println!("terminating");
    }
}
