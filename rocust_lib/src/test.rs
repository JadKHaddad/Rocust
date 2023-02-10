use crate::{
    results::{AllResults, ResultMessage},
    traits::{HasTask, PrioritisedRandom, User},
};
use prettytable::{row, Table};
use rand::Rng;
use std::{sync::Arc, time::Duration};
use tokio::sync::{Notify, RwLock};
use tokio::{sync::mpsc, time::Instant};
pub struct Test {
    pub user_count: u64,
    pub users_per_second: u64,
    pub runtime: Option<u64>,
    pub notify: Arc<Notify>,
    pub all_results_arc_rwlock: Arc<RwLock<AllResults>>,
    pub start_timestamp_arc_rwlock: Arc<RwLock<Instant>>,
}

impl Test {
    pub fn new(user_count: u64, users_per_second: u64, runtime: Option<u64>) -> Self {
        Test {
            user_count,
            users_per_second,
            runtime,
            notify: Arc::new(Notify::new()),
            all_results_arc_rwlock: Arc::new(RwLock::new(AllResults::default())),
            start_timestamp_arc_rwlock: Arc::new(RwLock::new(Instant::now())),
        }
    }

    fn calculate_elapsed_time(start_timestamp: &Instant) -> Duration {
        Instant::now().duration_since(*start_timestamp)
    }

    async fn sleep_between(between: (u64, u64)) {
        let between = rand::thread_rng().gen_range(between.0..between.1);
        tokio::time::sleep(Duration::from_secs(between)).await;
    }

    pub async fn run<T>(&self)
    where
        T: HasTask + User + Default + Send + 'static,
    {
        let tasks = Arc::new(T::get_async_tasks());
        if tasks.is_empty() {
            println!("user has no tasks");
            return;
        }
        let between = T::get_between();

        //set timestamp
        *self.start_timestamp_arc_rwlock.write().await = Instant::now();

        //spawm users in other task
        let (results_tx, mut results_rx) = mpsc::unbounded_channel();
        let results_tx_clone = results_tx.clone();
        let users_per_second = self.users_per_second;
        let notify = self.notify.clone();
        let user_count = self.user_count;

        let users_spawn_handle = tokio::spawn(async move {
            let mut handles = vec![];
            let mut users_spawned = 0;
            for i in 0..user_count {
                let user_notify = notify.clone();
                let spawn_notify = user_notify.clone();
                let tasks = tasks.clone();
                let results_tx_clone = results_tx_clone.clone();
                let handle = tokio::spawn(async move {
                    let mut user = T::default();
                    user.set_sender(results_tx_clone);
                    user.on_create(i as u16);
                    user.on_start();
                    loop {
                        // get a random task
                        if let Some(task) = tasks.get_proioritised_random() {
                            // call it
                            let task_call_and_sleep = async {
                                task.call(&mut user).await;
                                // this is the sleep time of a user
                                Test::sleep_between(between).await;
                            };
                            // do some sleep or stop
                            tokio::select! {
                                _ = user_notify.notified() => {
                                    break;
                                }
                                _ = task_call_and_sleep => {
                                }
                            }
                        }
                    }
                    user.on_stop();
                });
                handles.push(handle);
                users_spawned += 1;
                if users_spawned % users_per_second == 0 {
                    tokio::select! {
                        _ = spawn_notify.notified() => {
                            break;
                        }
                        _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                            users_spawned = 0;
                        }
                    }
                }
            }
            handles
        });

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
        //start the background tasks in another task (calculating stats, printing stats, managing files)
        let notify = self.notify.clone();
        let all_results_arc_rwlock = self.all_results_arc_rwlock.clone();
        let start_timestamp_arc_rwlock = self.start_timestamp_arc_rwlock.clone();
        let background_tasks_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = notify.notified() => {
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {
                        let mut all_results_gaurd = all_results_arc_rwlock.write().await;
                        //update stats
                        let elapsed_time = Test::calculate_elapsed_time(&*start_timestamp_arc_rwlock.read().await);
                        all_results_gaurd.calculate_per_second(&elapsed_time);
                        //print stats
                        let mut table = Table::new();
                        table.add_row(row![
                            "TYPE",
                            "NAME",
                            "TOTAL REQ",
                            "FAILED REQ",
                            "TOTAL ERR",
                            "REQ/S",
                            "FAILED REQ/S",
                            "TOTAL RES TIME",
                            "AVG RES TIME",
                            "MIN RES TIME",
                            "MAX RES TIME",
                        ]);
                        for (endpoint_type_name, results) in &all_results_gaurd.endpoint_results {
                            table.add_row(row![
                                endpoint_type_name.0,
                                endpoint_type_name.1,
                                results.total_requests,
                                results.total_failed_requests,
                                results.total_errors,
                                results.requests_per_second,
                                results.failed_requests_per_second,
                                results.total_response_time,
                                results.average_response_time,
                                results.min_response_time,
                                results.max_response_time,
                            ]);
                        }
                        table.add_row(row![
                            " ",
                            "AGR",
                            all_results_gaurd.aggrigated_results.total_requests,
                            all_results_gaurd.aggrigated_results.total_failed_requests,
                            all_results_gaurd.aggrigated_results.total_errors,
                            all_results_gaurd.aggrigated_results.requests_per_second,
                            all_results_gaurd.aggrigated_results.failed_requests_per_second,
                            all_results_gaurd.aggrigated_results.total_response_time,
                            all_results_gaurd.aggrigated_results.average_response_time,
                            all_results_gaurd.aggrigated_results.min_response_time,
                            all_results_gaurd.aggrigated_results.max_response_time,
                        ]);
                        table.printstd();
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
        if let Ok(handles) = users_spawn_handle.await {
            for handle in handles {
                handle.await.unwrap();
            }
        }
        println!("all users finished");
        background_tasks_handle.await.unwrap();
        timer_handle.await.unwrap();
        println!("terminating");
    }
}
