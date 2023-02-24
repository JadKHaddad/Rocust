use crate::{
    data::{Data, StopConditionData},
    events::EventsHandler,
    messages::{MainMessage, ResultMessage},
    results::AllResults,
    server::Server,
    test_config::TestConfig,
    traits::{HasTask, PrioritisedRandom, Shared, User},
    user::{UserController, UserInfo, UserPanicInfo},
    utils::get_timestamp_as_millis_as_string,
    writer::Writer,
};
use rand::Rng;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    io::{self, AsyncWriteExt},
    sync::{mpsc, RwLock},
    task::JoinHandle,
    time::Instant,
};
use tokio_util::sync::CancellationToken;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    filter::EnvFilter, fmt, prelude::__tracing_subscriber_SubscriberExt, Layer,
};

#[derive(Clone)]
pub struct TestController {
    token: Arc<CancellationToken>,
}

impl TestController {
    pub fn new(token: Arc<CancellationToken>) -> Self {
        TestController { token }
    }

    pub fn stop(&self) {
        tracing::info!("Stopping test");
        self.token.cancel();
    }

    pub(crate) async fn cancelled(&self) {
        self.token.cancelled().await
    }
}

pub struct Test {
    test_config: TestConfig,
    token: Arc<CancellationToken>,
    current_results_writer: Option<Writer>,
    results_history_writer: Option<Writer>,
    total_users_spawned_arc_rwlock: Arc<RwLock<u64>>,
    all_results_arc_rwlock: Arc<RwLock<AllResults>>,
    users_results_arc_rwlock: Arc<RwLock<HashMap<u64, AllResults>>>,
    start_timestamp_arc_rwlock: Arc<RwLock<Instant>>,
}

impl Test {
    pub async fn new(test_config: TestConfig) -> Self {
        let current_results_writer = if let Some(current_results_file) =
            &test_config.current_results_file
        {
            match Writer::from_str(current_results_file).await {
                Ok(writer) => Some(writer),
                Err(e) => {
                    tracing::error!("Failed to create writer for current results file: [{}]", e);
                    None
                }
            }
        } else {
            None
        };
        let results_history_writer = if let Some(results_history_file) =
            &test_config.results_history_file
        {
            match Writer::from_str(results_history_file).await {
                Ok(writer) => {
                    // write header
                    let header = AllResults::history_header_csv_string();
                    match header {
                        Ok(header) => match writer.write_all(header.as_bytes()).await {
                            Ok(_) => Some(writer),
                            Err(e) => {
                                tracing::error!(
                                    "Failed to write header to results history file: [{}]",
                                    e
                                );
                                None
                            }
                        },
                        Err(e) => {
                            tracing::error!(
                                "Failed to create header for results history file: [{}]",
                                e
                            );
                            None
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create writer for results history file: [{}]", e);
                    None
                }
            }
        } else {
            None
        };
        Test {
            test_config,
            token: Arc::new(CancellationToken::new()),
            current_results_writer,
            results_history_writer,
            total_users_spawned_arc_rwlock: Arc::new(RwLock::new(0)),
            all_results_arc_rwlock: Arc::new(RwLock::new(AllResults::default())),
            users_results_arc_rwlock: Arc::new(RwLock::new(HashMap::new())),
            start_timestamp_arc_rwlock: Arc::new(RwLock::new(Instant::now())),
        }
    }

    pub fn create_test_controller(&self) -> TestController {
        TestController::new(self.token.clone())
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

    // TODO: this is a bit of a mess, clean it up
    // TODO: check geven logfile
    // TODO: check if print to stdout
    pub fn setup_logging(&self) -> WorkerGuard {
        let file_appender = tracing_appender::rolling::never("results", "prefix.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        match self.test_config.log_level {
            Some(log_level) => {
                let subscriber = tracing_subscriber::registry()
                    .with(
                        fmt::layer()
                            .with_writer(std::io::stdout)
                            .with_filter(log_level),
                    )
                    .with(
                        fmt::layer()
                            .with_writer(non_blocking)
                            .with_filter(log_level),
                    );

                if let Err(_) = tracing::subscriber::set_global_default(subscriber) {
                    tracing::warn!("Failed to set global default subscriber");
                }
            }
            None => {
                let subscriber = tracing_subscriber::registry()
                    .with(
                        fmt::layer()
                            .with_writer(std::io::stdout)
                            .with_filter(EnvFilter::from_env("ROCUST_LOG")),
                    )
                    .with(
                        fmt::layer()
                            .with_writer(non_blocking)
                            .with_filter(EnvFilter::from_env("ROCUST_LOG")),
                    );

                if let Err(_) = tracing::subscriber::set_global_default(subscriber) {
                    tracing::warn!("Failed to set global default subscriber");
                }
            }
        }
        guard
    }

    pub fn spawn_users<T, S>(
        &self,
        count: u64,
        starting_index: u64,
        results_tx: mpsc::UnboundedSender<MainMessage>,
        test_controller: Arc<TestController>,
        shared: S,
    ) -> JoinHandle<Vec<(JoinHandle<UserInfo>, UserPanicInfo)>>
    where
        T: HasTask + User + User<Shared = S> + 'static,
        S: Shared + 'static,
    {
        let tasks = Arc::new(T::get_async_tasks());
        if tasks.is_empty() {
            tracing::warn!("User [{}] has no tasks. Will not be spawned", T::get_name());
            return tokio::spawn(async move { vec![] }); // just to avoid an infinite loop
        }
        let between = T::get_between();

        let token = self.token.clone();

        let test_config = self.test_config.clone();
        let users_per_sec = test_config.users_per_sec;
        tokio::spawn(async move {
            let mut handles = vec![];
            let mut users_spawned = 0;
            for i in 0..count {
                let test_config = test_config.clone();
                let id = i as u64 + starting_index;

                // these are the tokens for the test
                let test_token_for_user = token.clone();
                let test_spawn_token = token.clone();

                // create a user token for the UserController
                let user_token = Arc::new(CancellationToken::new());
                let user_spawn_token = user_token.clone();
                let user_controller = UserController::new(id.clone(), user_token.clone());
                let events_handler = EventsHandler::new(id.clone(), results_tx.clone());

                // create the data for the user
                let user_data = Data::new(
                    test_controller.clone(),
                    events_handler.clone(),
                    user_controller,
                );

                let tasks = tasks.clone();
                let shared = shared.clone();

                let handle = tokio::spawn(async move {
                    let mut user = T::new(&test_config, &user_data, shared).await;
                    let mut total_tasks: u64 = 0;
                    user.on_start(&user_data).await;

                    loop {
                        // get a random task
                        if let Some(task) = tasks.get_proioritised_random() {
                            // call it, do some sleep
                            let task_call_and_sleep = async {
                                // this is the sleep time of a user
                                Test::sleep_between(between).await;

                                // this is the actual task
                                task.call(&mut user, &user_data).await;

                                total_tasks += 1;
                            };

                            tokio::select! {
                                _ = user_token.cancelled() => {
                                    tracing::info!("User [{}][{}] attempted suicide", T::get_name(), id);
                                    break;
                                }
                                _ = test_token_for_user.cancelled() => {
                                    break;
                                }
                                _ = task_call_and_sleep => {
                                }
                            }
                        }
                    }

                    user.on_stop(&user_data).await;
                    UserInfo::new(id, T::get_name(), total_tasks)
                });
                handles.push((handle, UserPanicInfo::new(id, T::get_name())));
                events_handler.add_user_spawned(id, T::get_name());
                users_spawned += 1;
                if users_spawned % users_per_sec == 0 {
                    tokio::select! {
                        _ = user_spawn_token.cancelled() => {
                            tracing::info!("User [{}][{}] attempted suicide", T::get_name(), id);
                            break;
                        }
                        _ = test_spawn_token.cancelled() => {
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
        let test_controller = self.create_test_controller();
        let all_results_arc_rwlock = self.all_results_arc_rwlock.clone();
        let addr = self.test_config.server_address;
        match addr {
            Some(addr) => {
                tracing::info!("Server listening on [{}]", addr);
                tokio::spawn(async move {
                    let server = Server::new(test_controller, all_results_arc_rwlock, addr);
                    // no tokio::select! here because axum is running with graceful shutdown
                    let res = server.run().await;
                    if let Err(e) = res {
                        tracing::error!("Server error: {}", e);
                    }
                })
            }
            None => {
                tracing::info!("Server disabled");
                tokio::spawn(async move {})
            }
        }
    }

    fn start_background_tasks(&self, total_spawnable_user_count: u64) -> JoinHandle<()> {
        // note: not updating stats for every user, only for the whole test. user stats are only updated once when summery is created
        let token = self.token.clone();
        let total_users_spawned_arc_rwlock = self.total_users_spawned_arc_rwlock.clone();
        let all_results_arc_rwlock = self.all_results_arc_rwlock.clone();
        let start_timestamp_arc_rwlock = self.start_timestamp_arc_rwlock.clone();
        let test_config = self.test_config.clone();
        let current_results_writer = self.current_results_writer.clone();
        let results_history_writer = self.results_history_writer.clone();
        tokio::spawn(async move {
            let mut print_total_spawned_users = true;
            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(test_config.update_interval_in_secs)) => {

                        if print_total_spawned_users {
                            let total_users_spawned_gaurd = total_users_spawned_arc_rwlock.read().await;
                            tracing::info!("Total users spawned: [{}/{}]", *total_users_spawned_gaurd, total_spawnable_user_count);
                            if *total_users_spawned_gaurd == total_spawnable_user_count {
                                tracing::info!("All users spawned [{}]", total_spawnable_user_count);
                                print_total_spawned_users = false;
                            }
                        }

                        let mut all_results_gaurd = all_results_arc_rwlock.write().await;

                        // update stats
                        let elapsed_time = Test::calculate_elapsed_time(&*start_timestamp_arc_rwlock.read().await);
                        all_results_gaurd.calculate_per_second(&elapsed_time);

                        // print stats
                        if test_config.print_to_stdout {
                            let table_string = all_results_gaurd.table_string();
                            let mut stdout = io::stdout();
                            let _ = stdout.write_all(table_string.as_bytes()).await;
                        }

                        // write current results to csv
                        if let Some(writer) = &current_results_writer {
                            let csv_string = all_results_gaurd.current_results_csv_string();
                            match csv_string {
                                Ok(csv_string) => {
                                    match writer.write_all(csv_string.as_bytes()).await {
                                        Ok(_) => {}
                                        Err(e) => {
                                            tracing::error!("Error writing to csv: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Error getting csv string: {}", e);
                                }
                            }
                        }

                        // write results history to csv
                        if let Some(writer) = &results_history_writer {
                            match get_timestamp_as_millis_as_string() {
                                Ok(timestamp) => {
                                    let csv_string = all_results_gaurd.current_aggrigated_results_with_timestamp_csv_string(&timestamp);
                                    match csv_string {
                                        Ok(csv_string) => {
                                            match writer.append_all(csv_string.as_bytes()).await {
                                                Ok(_) => {}
                                                Err(e) => {
                                                    tracing::error!("Error writing to csv: {}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Error getting csv string: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Error getting timestamp: {}", e);
                                }
                            }
                        }

                        // check stop condition and stop if needed
                        if let Some(stop_condition) = &test_config.stop_condition {
                            let stop_condition_data = StopConditionData::new(&*all_results_gaurd, &elapsed_time);
                            if stop_condition(stop_condition_data) {
                                tracing::info!("Stop condition met");
                                token.cancel();
                            }
                        }
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
                    let mut users_results_gaurd = self.users_results_arc_rwlock.write().await;
                    match result_msg {
                        ResultMessage::Success(sucess_result_msg) => {
                            all_results_gaurd.add_success(
                                &sucess_result_msg.endpoint_type_name,
                                sucess_result_msg.response_time,
                            );
                            // updating user results
                            if let Some(user_all_results) =
                                users_results_gaurd.get_mut(&sucess_result_msg.user_id)
                            {
                                user_all_results.add_success(
                                    &sucess_result_msg.endpoint_type_name,
                                    sucess_result_msg.response_time,
                                );
                            }
                        }
                        ResultMessage::Failure(failure_result_msg) => {
                            all_results_gaurd.add_failure(&failure_result_msg.endpoint_type_name);

                            // updating user results
                            if let Some(user_all_results) =
                                users_results_gaurd.get_mut(&failure_result_msg.user_id)
                            {
                                user_all_results
                                    .add_failure(&failure_result_msg.endpoint_type_name);
                            }
                        }
                        ResultMessage::Error(error_result_msg) => {
                            all_results_gaurd.add_error(
                                &error_result_msg.endpoint_type_name,
                                &error_result_msg.error,
                            );

                            // updating user results
                            if let Some(user_all_results) =
                                users_results_gaurd.get_mut(&error_result_msg.user_id)
                            {
                                user_all_results.add_error(
                                    &error_result_msg.endpoint_type_name,
                                    &error_result_msg.error,
                                );
                            }
                        }
                    }
                }
                MainMessage::UserSpawned(user_spawned_msg) => {
                    let mut total_users_spawned_gaurd =
                        self.total_users_spawned_arc_rwlock.write().await;
                    *total_users_spawned_gaurd += 1;

                    let mut users_results_gaurd = self.users_results_arc_rwlock.write().await;
                    users_results_gaurd.insert(user_spawned_msg.id, AllResults::default());
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
        results_rx: mpsc::UnboundedReceiver<MainMessage>,
        spawn_users_handles_vec: Vec<JoinHandle<Vec<(JoinHandle<UserInfo>, UserPanicInfo)>>>,
        total_spawnable_user_count: u64,
    ) {
        // spin up a server
        let server_handle = self.strat_server();

        // start a timer in another task
        let timer_handle = self.start_timer();

        // start the background tasks in another task (calculating stats, printing stats, managing files)
        let background_tasks_handle = self.start_background_tasks(total_spawnable_user_count);

        // start the reciever
        self.block_on_reciever(results_rx).await;
        tracing::debug!("Reciever dropped");

        // this will cancel the timer and background tasks if the only given user has no tasks so it will finish immediately thus causing the reciever to drop
        self.token.cancel();

        // wait for all users to finish
        for spawn_users_handles in spawn_users_handles_vec {
            match spawn_users_handles.await {
                Ok(handles) => {
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
                                        "User [{}][{}] panic!(ed)",
                                        user_panic_info.name,
                                        user_panic_info.id
                                    )
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error joining users: {}", e);
                }
            }
        }
        if let Err(e) = server_handle.await {
            tracing::error!("Error joining server: {}", e);
        }
        tracing::debug!("Server finished");

        if let Err(e) = background_tasks_handle.await {
            tracing::error!("Error joining background tasks: {}", e);
        }
        tracing::debug!("Background tasks finished");

        if let Err(e) = timer_handle.await {
            tracing::error!("Error joining timer: {}", e);
        }
        tracing::debug!("Timer finished");

        tracing::info!("Test finished");

        // TODO: remove dev
        // Dev: print all results of all users. but lets do a quick update of the results
        let elapsed_time =
            Test::calculate_elapsed_time(&*self.start_timestamp_arc_rwlock.read().await);
        for (user_id, user_results) in self.users_results_arc_rwlock.write().await.iter_mut() {
            user_results.calculate_per_second(&elapsed_time);
            println!("User [{}]", user_id);
            println!("{:#?}", user_results);
            println!("--------------------------------");
        }
    }
}

impl Drop for Test {
    // test is not clone so this is fine to stop the test on drop
    fn drop(&mut self) {
        self.token.cancel();
    }
}
