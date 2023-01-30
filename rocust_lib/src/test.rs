use crate::{
    results::{AllResults, ResultMessage},
    traits::{HasTask, PrioritisedRandom, User},
};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Notify, RwLock,
};

pub struct Test {
    pub user_count: i32,
    pub runtime: Option<u32>,
    pub notify: Arc<Notify>,
    pub all_results_arc_rwlock: Arc<RwLock<AllResults>>,
    pub results_tx: Sender<ResultMessage>,
    pub results_rx: Receiver<ResultMessage>,
}

impl Test {
    pub fn new(user_count: i32, runtime: Option<u32>) -> Self {
        let (results_tx, results_rx) = mpsc::channel(1024);
        Test {
            user_count,
            runtime,
            notify: Arc::new(Notify::new()),
            all_results_arc_rwlock: Arc::new(RwLock::new(AllResults::default())),
            results_tx,
            results_rx,
        }
    }

    pub async fn run<T>(&mut self)
    where
        T: HasTask + User + Default + Send + 'static,
    {
        let mut handles = vec![];
        let tasks = Arc::new(T::get_async_tasks());
        if tasks.is_empty() {
            println!("user has no tasks");
            return;
        }

        for i in 0..self.user_count {
            //control the spawn rate
            let notify = self.notify.clone();
            let tasks = tasks.clone();
            let handle = tokio::spawn(async move {
                let mut user = T::default();
                user.on_create(i as u16);
                user.on_start();
                loop {
                    // get a random task
                    let task = tasks.get_proioritised_random().unwrap();
                    // call it
                    let task_call_and_sleep = async {
                        task.call(&mut user).await;
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
                    _ = tokio::time::sleep(std::time::Duration::from_secs(runtime as u64)) => {
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

        //start the reciever
        while let Some(result_msg) = self.results_rx.recv().await {
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

        timer_handle.await.unwrap();
        println!("terminating");
    }
}
