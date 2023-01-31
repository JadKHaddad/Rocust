use crate::{
    results::{AllResults, ResultMessage},
    traits::{HasTask, PrioritisedRandom, User},
};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::{Notify, RwLock};

pub struct Test {
    pub user_count: u64,
    pub runtime: Option<u64>,
    pub notify: Arc<Notify>,
    pub all_results_arc_rwlock: Arc<RwLock<AllResults>>,
}

impl Test {
    pub fn new(user_count: u64, runtime: Option<u64>) -> Self {
        Test {
            user_count,
            runtime,
            notify: Arc::new(Notify::new()),
            all_results_arc_rwlock: Arc::new(RwLock::new(AllResults::default())),
        }
    }

    pub async fn run<T>(&self)
    where
        T: HasTask + User + Default + Send + 'static,
    {
        let mut handles = vec![];
        let tasks = Arc::new(T::get_async_tasks());
        if tasks.is_empty() {
            println!("user has no tasks");
            return;
        }
        let (results_tx, mut results_rx) = mpsc::unbounded_channel();
        for i in 0..self.user_count {
            //control the spawn rate
            let notify = self.notify.clone();
            let tasks = tasks.clone();
            let results_tx = results_tx.clone();
            let handle = tokio::spawn(async move {
                let mut user = T::default();
                user.set_sender(results_tx);
                user.on_create(i as u16);
                user.on_start();
                loop {
                    // get a random task
                    let task = tasks.get_proioritised_random().unwrap();
                    // call it
                    let task_call_and_sleep = async {
                        task.call(&mut user).await;
                        // this is the sleep time of a user
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
        let timer_handle = if let Some(runtime) = self.runtime {
            println!("runtime: {}s", runtime);
            tokio::spawn(async move {
                tokio::select! {
                    // this is the ctrl+c or any other signal
                    _ = notify.notified() => {
                        println!("received signal");
                    }
                    // this is the run time
                    _ = tokio::time::sleep(std::time::Duration::from_secs(runtime)) => {
                        println!("timer finished");
                        notify.notify_waiters();
                    }
                }
            })
        } else {
            println!("runtime: infinite");
            tokio::spawn(async move {
                // this is the ctrl+c or any other signal
                notify.notified().await;
            })
        };
        //start the printer in another task
        let notify = self.notify.clone();
        let all_results_arc_rwlock = self.all_results_arc_rwlock.clone();
        let printer_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = notify.notified() => {
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {
                        let all_results_gaurd = all_results_arc_rwlock.read().await;
                        println!("******** results ********");
                        println!("{:?}", all_results_gaurd);
                        println!("*************************");
                    }
                }
            }
        });
        //drop the sender so the reciever will terminate when all users are done
        drop(results_tx);
        //start the reciever
        while let Some(result_msg) = results_rx.recv().await {
            let mut all_results_gaurd = self.all_results_arc_rwlock.write().await;
            match result_msg {
                ResultMessage::Success(sucess_result_msg) => {
                    all_results_gaurd.add_success(
                        sucess_result_msg.endpoint_type_name,
                        sucess_result_msg.response_time,
                    );
                }
                ResultMessage::Failure(failure_result_msg) => {
                    all_results_gaurd.add_failure(failure_result_msg.endpoint_type_name);
                }
                ResultMessage::Error(error_result_msg) => {
                    all_results_gaurd
                        .add_error(error_result_msg.endpoint_type_name, error_result_msg.error);
                }
            }
        }
        println!("reciever dropped");
        for handle in handles {
            handle.await.unwrap();
        }
        println!("all users finished");
        printer_handle.await.unwrap();
        timer_handle.await.unwrap();
        println!("terminating");
    }
}
