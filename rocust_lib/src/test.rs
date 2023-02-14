use crate::traits::Shared;
use crate::{
    results::{AllResults, EventsHandler, ResultMessage},
    traits::{HasTask, PrioritisedRandom, User},
};
use rand::Rng;
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{mpsc, RwLock},
    task::JoinHandle,
    time::Instant,
};
use tokio_util::sync::CancellationToken;
pub struct Test {
    pub user_count: u64,
    pub users_per_second: u64,
    pub runtime: Option<u64>,
    pub token: Arc<CancellationToken>,
    pub all_results_arc_rwlock: Arc<RwLock<AllResults>>,
    pub start_timestamp_arc_rwlock: Arc<RwLock<Instant>>,
}

impl Test {
    pub fn new(user_count: u64, users_per_second: u64, runtime: Option<u64>) -> Self {
        Test {
            user_count,
            users_per_second,
            runtime,
            token: Arc::new(CancellationToken::new()),
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

    pub fn spawn_users<T, S>(
        &self,
        event_handler: EventsHandler,
        shared: S,
    ) -> JoinHandle<Vec<JoinHandle<()>>>
    where
        T: HasTask + User + User<Shared = S> + 'static,
        S: Shared + 'static,
    {
        let tasks = Arc::new(T::get_async_tasks());
        if tasks.is_empty() {
            println!("Warning user has no tasks");
            return tokio::spawn(async move { vec![] }); // just to avoid an infinite loop
        }
        let between = T::get_between();
        let users_per_second = self.users_per_second;
        let token = self.token.clone();
        let user_count = self.user_count;
        tokio::spawn(async move {
            let mut handles = vec![];
            let mut users_spawned = 0;
            for i in 0..user_count {
                let event_handler = event_handler.clone();
                let user_token = token.clone();
                let spawn_token = user_token.clone();
                let tasks = tasks.clone();
                let shared = shared.clone();
                let handle = tokio::spawn(async move {
                    let mut user = T::new(i as u16, &event_handler, shared);
                    user.on_start(&event_handler);
                    loop {
                        // get a random task
                        if let Some(task) = tasks.get_proioritised_random() {
                            // call it
                            let task_call_and_sleep = async {
                                // this is the sleep time of a user
                                Test::sleep_between(between).await;
                                // this is the actual task
                                task.call(&mut user, &event_handler).await;
                            };
                            // do some sleep or stop
                            tokio::select! {
                                _ = user_token.cancelled() => {
                                    break;
                                }
                                _ = task_call_and_sleep => {
                                }
                            }
                        }
                    }
                    user.on_stop(&event_handler);
                });
                handles.push(handle);
                users_spawned += 1;
                if users_spawned % users_per_second == 0 {
                    tokio::select! {
                        _ = spawn_token.cancelled() => {
                            break;
                        }
                        _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                            users_spawned = 0;
                        }
                    }
                }
            }
            handles
        })
    }

    fn start_timer(&self) -> JoinHandle<()> {
        let token = self.token.clone();
        match self.runtime {
            Some(runtime) => {
                println!("runtime: {}s", runtime);
                tokio::spawn(async move {
                    tokio::select! {
                        // this is the ctrl+c or any other signal
                        _ = token.cancelled() => {
                            println!("received signal");
                        }
                        // this is the run time
                        _ = tokio::time::sleep(std::time::Duration::from_secs(runtime)) => {
                            println!("timer finished");
                            token.cancel();
                        }
                    }
                })
            }
            None => {
                println!("runtime: infinite");
                tokio::spawn(async move {
                    // this is the ctrl+c or any other signal
                    token.cancelled().await;
                })
            }
        }
    }

    fn start_background_tasks(&self) -> JoinHandle<()> {
        let token = self.token.clone();
        let all_results_arc_rwlock = self.all_results_arc_rwlock.clone();
        let start_timestamp_arc_rwlock = self.start_timestamp_arc_rwlock.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {
                        let mut all_results_gaurd = all_results_arc_rwlock.write().await;
                        //update stats
                        let elapsed_time = Test::calculate_elapsed_time(&*start_timestamp_arc_rwlock.read().await);
                        all_results_gaurd.calculate_per_second(&elapsed_time);
                        //print stats
                        all_results_gaurd.print_table();
                    }
                }
            }
        })
    }

    async fn block_on_reciever(&self, mut results_rx: mpsc::UnboundedReceiver<ResultMessage>) {
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
    }

    pub async fn before_spawn_users(
        &self,
    ) -> (
        mpsc::UnboundedSender<ResultMessage>,
        mpsc::UnboundedReceiver<ResultMessage>,
    ) {
        //set timestamp
        *self.start_timestamp_arc_rwlock.write().await = Instant::now();
        mpsc::unbounded_channel()
    }

    pub async fn after_spawn_users(
        &self,
        events_handler: EventsHandler,
        results_rx: mpsc::UnboundedReceiver<ResultMessage>,
        spawn_users_handles_vec: Vec<JoinHandle<Vec<JoinHandle<()>>>>,
    ) {
        //start a timer in another task
        let timer_handle = self.start_timer();

        //start the background tasks in another task (calculating stats, printing stats, managing files)
        let background_tasks_handle = self.start_background_tasks();

        //drop the events_handler to drop the sender sender
        drop(events_handler);

        //start the reciever
        self.block_on_reciever(results_rx).await;
        println!("reciever dropped");

        //wait for all users to finish
        for spawn_users_handles in spawn_users_handles_vec {
            if let Ok(handles) = spawn_users_handles.await {
                for handle in handles {
                    handle.await.unwrap();
                }
            }
        }

        println!("all users finished");

        background_tasks_handle.await.unwrap();
        println!("background tasks finished");

        timer_handle.await.unwrap();
        println!("timer finished");

        println!("terminating");
    }
}
