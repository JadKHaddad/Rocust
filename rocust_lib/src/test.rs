use crate::{
    events::EventsHandler,
    messages::{MainMessage, ResultMessage, UserSpawnedMessage},
    results::AllResults,
    server::Server,
    traits::{HasTask, PrioritisedRandom, Shared, User},
    user::{UserInfo, UserPanicInfo},
};
use rand::Rng;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    io::{self, AsyncWriteExt},
    sync::{mpsc, RwLock},
    task::JoinHandle,
    time::Instant,
};
use tokio_util::sync::CancellationToken;

pub struct TestConfig {
    pub user_count: u64,
    pub users_per_second: u64,
    pub runtime: Option<u64>,
    pub update_interval_in_secs: u64,
    pub addr: SocketAddr,
}

impl TestConfig {
    pub fn new(
        user_count: u64,
        users_per_second: u64,
        runtime: Option<u64>,
        update_interval_in_secs: u64,
        addr: SocketAddr,
    ) -> Self {
        TestConfig {
            user_count,
            users_per_second,
            runtime,
            update_interval_in_secs,
            addr,
        }
    }
}

#[derive(Clone)]
pub struct TestController {
    pub(crate) token: Arc<CancellationToken>,
    all_results_arc_rwlock: Arc<RwLock<AllResults>>,
}

impl TestController {
    pub fn new(
        token: Arc<CancellationToken>,
        all_results_arc_rwlock: Arc<RwLock<AllResults>>,
    ) -> Self {
        TestController {
            token,
            all_results_arc_rwlock,
        }
    }

    pub fn stop(&self) {
        tracing::info!("Stopping test");
        self.token.cancel();
    }

    pub async fn get_results(&self) -> AllResults {
        self.all_results_arc_rwlock.read().await.clone()
    }
}

pub struct Test {
    test_config: TestConfig,
    token: Arc<CancellationToken>,
    total_users_spawned_arc_rwlock: Arc<RwLock<u64>>,
    all_results_arc_rwlock: Arc<RwLock<AllResults>>,
    start_timestamp_arc_rwlock: Arc<RwLock<Instant>>,
}

impl Test {
    pub fn new(test_config: TestConfig) -> Self {
        Test {
            test_config,
            token: Arc::new(CancellationToken::new()),
            total_users_spawned_arc_rwlock: Arc::new(RwLock::new(0)),
            all_results_arc_rwlock: Arc::new(RwLock::new(AllResults::default())),
            start_timestamp_arc_rwlock: Arc::new(RwLock::new(Instant::now())),
        }
    }

    pub fn get_controller(&self) -> TestController {
        TestController::new(self.token.clone(), self.all_results_arc_rwlock.clone())
    }

    pub fn get_config(&self) -> &TestConfig {
        &self.test_config
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
        count: u64,
        starting_index: u64,
        event_handler: EventsHandler,
        shared: S,
    ) -> JoinHandle<Vec<(JoinHandle<UserInfo>, UserPanicInfo)>>
    where
        T: HasTask + User + User<Shared = S> + 'static,
        S: Shared + 'static,
    {
        let tasks = Arc::new(T::get_async_tasks());
        if tasks.is_empty() {
            tracing::warn!("User [{}] has no tasks", T::get_name());
            return tokio::spawn(async move { vec![] }); // just to avoid an infinite loop
        }
        let between = T::get_between();
        let users_per_second = self.test_config.users_per_second;
        let token = self.token.clone();
        let user_count = count;
        tokio::spawn(async move {
            let mut handles = vec![];
            let mut users_spawned = 0;
            for i in 0..user_count {
                let id = i as u64 + starting_index;
                let user_event_handler = event_handler.clone();
                let user_token = token.clone();
                let spawn_token = user_token.clone();
                let tasks = tasks.clone();
                let shared = shared.clone();
                let handle = tokio::spawn(async move {
                    let mut user = T::new(id, &user_event_handler, shared);
                    let mut total_tasks: u64 = 0;
                    user.on_start(&user_event_handler);
                    loop {
                        // get a random task
                        if let Some(task) = tasks.get_proioritised_random() {
                            // call it
                            let task_call_and_sleep = async {
                                // this is the sleep time of a user
                                Test::sleep_between(between).await;
                                // this is the actual task
                                task.call(&mut user, &user_event_handler).await;
                                total_tasks += 1;
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
                    user.on_stop(&user_event_handler);
                    UserInfo::new(id, T::get_name(), total_tasks)
                });
                handles.push((handle, UserPanicInfo::new(id, T::get_name())));
                let _ = event_handler
                    .sender
                    .send(MainMessage::UserSpawned(UserSpawnedMessage {
                        id,
                        name: T::get_name(),
                    }));
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
        match self.test_config.runtime {
            Some(runtime) => {
                tracing::info!("Runtime: {}s", runtime);
                tokio::spawn(async move {
                    tokio::select! {
                        // this could be ctrl+c or any other signal
                        _ = token.cancelled() => {
                            tracing::info!("Received signal");
                        }
                        // this is the run time
                        _ = tokio::time::sleep(std::time::Duration::from_secs(runtime)) => {
                            tracing::debug!("Timer finished");
                            token.cancel();
                        }
                    }
                })
            }
            None => {
                tracing::info!("Runtime: infinite");
                tokio::spawn(async move {
                    // this could be ctrl+c or any other signal
                    token.cancelled().await;
                    tracing::info!("Received signal");
                })
            }
        }
    }

    fn strat_server(&self) -> JoinHandle<()> {
        let test_controller = self.get_controller().clone();
        let addr = self.test_config.addr;
        tokio::spawn(async move {
            let server = Server::new(test_controller, addr);
            tracing::info!("Listening on {}", addr);
            // no tokio::select! here because axum is running with graceful shutdown
            let res = server.run().await;
            if let Err(e) = res {
                tracing::error!("Server error: {}", e);
            }
        })
    }

    fn start_background_tasks(&self) -> JoinHandle<()> {
        let token = self.token.clone();
        let total_users_spawned_arc_rwlock = self.total_users_spawned_arc_rwlock.clone();
        let all_results_arc_rwlock = self.all_results_arc_rwlock.clone();
        let start_timestamp_arc_rwlock = self.start_timestamp_arc_rwlock.clone();
        let update_interval_in_secs = self.test_config.update_interval_in_secs;
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(update_interval_in_secs)) => {

                        let total_users_spawned_gaurd = total_users_spawned_arc_rwlock.read().await;
                        tracing::info!("Total users spawned: [{}]", *total_users_spawned_gaurd);

                        let mut all_results_gaurd = all_results_arc_rwlock.write().await;
                        // update stats
                        let elapsed_time = Test::calculate_elapsed_time(&*start_timestamp_arc_rwlock.read().await);
                        all_results_gaurd.calculate_per_second(&elapsed_time);
                        // print stats

                        let table_string = all_results_gaurd.table_string();
                        let mut stdout = io::stdout();
                        let _ = stdout.write_all(table_string.as_bytes()).await;

                        // let csv_string = all_results_gaurd.csv_string();
                        // let mut stdout = io::stdout();
                        // let _ = stdout.write_all(csv_string.as_bytes()).await;
                    }
                }
            }
        })
    }

    async fn block_on_reciever(&self, mut results_rx: mpsc::UnboundedReceiver<MainMessage>) {
        while let Some(msg) = results_rx.recv().await {
            match msg {
                MainMessage::ResultMessage(result_msg) => {
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
                            all_results_gaurd.add_error(
                                error_result_msg.endpoint_type_name,
                                error_result_msg.error,
                            );
                        }
                    }
                }
                MainMessage::UserSpawned(_) => {
                    let mut total_users_spawned_gaurd =
                        self.total_users_spawned_arc_rwlock.write().await;
                    *total_users_spawned_gaurd += 1;
                }
            }
        }
    }

    pub async fn before_spawn_users(
        &self,
    ) -> (
        mpsc::UnboundedSender<MainMessage>,
        mpsc::UnboundedReceiver<MainMessage>,
    ) {
        // set timestamp
        *self.start_timestamp_arc_rwlock.write().await = Instant::now();
        mpsc::unbounded_channel()
    }

    pub async fn after_spawn_users(
        &self,
        events_handler: EventsHandler,
        results_rx: mpsc::UnboundedReceiver<MainMessage>,
        spawn_users_handles_vec: Vec<JoinHandle<Vec<(JoinHandle<UserInfo>, UserPanicInfo)>>>,
    ) {
        // spin up a server
        let server_handle = self.strat_server();

        // start a timer in another task
        let timer_handle = self.start_timer();

        // start the background tasks in another task (calculating stats, printing stats, managing files)
        let background_tasks_handle = self.start_background_tasks();

        // drop the events_handler to drop the sender
        drop(events_handler);

        // start the reciever
        self.block_on_reciever(results_rx).await;
        tracing::debug!("Reciever dropped");

        // this will cancel the timer and background tasks if the only given user has no tasks so it will finish immediately thus causing the reciever to drop
        self.token.cancel();

        // wait for all users to finish
        for spawn_users_handles in spawn_users_handles_vec {
            if let Ok(handles) = spawn_users_handles.await {
                for (handle, user_panic_info) in handles {
                    // TODO: create summary of all users and save it to a file
                    match handle.await {
                        Ok(user_info) => {
                            tracing::info!(
                                "User [{}][{}] finished with [{}] tasks",
                                user_info.name,
                                user_info.id,
                                user_info.total_tasks
                            )
                        }
                        Err(e) => {
                            if e.is_cancelled() {
                                tracing::warn!(
                                    "User [{}][{}] was cancelled",
                                    user_panic_info.name,
                                    user_panic_info.id
                                )
                            }
                            if e.is_panic() {
                                tracing::error!(
                                    "User [{}][{}] panicked",
                                    user_panic_info.name,
                                    user_panic_info.id
                                )
                            }
                        }
                    }
                }
            }
        }
        server_handle.await.unwrap();
        tracing::debug!("Server finished");

        background_tasks_handle.await.unwrap();
        tracing::debug!("Background tasks finished");

        timer_handle.await.unwrap();
        tracing::debug!("Timer finished");

        tracing::info!("Test finished");
    }
}

impl Drop for Test {
    // test is not clone so this is fine to stop the test on drop
    fn drop(&mut self) {
        self.token.cancel();
    }
}
