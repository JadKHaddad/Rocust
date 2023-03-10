pub mod config;
pub(crate) mod controller;
pub mod user;

use crate::{
    events::EventsHandler,
    fs::{timestamped_writer::TimeStapmedWriter, writer::Writer},
    logging::setup_logging,
    messages::{MainMessage, ResultMessage},
    prometheus_exporter::{PrometheusExporter, RequestLabel, TaskLabel, UserLabel},
    results::AllResults,
    server::Server,
    tasks::EventsTaskInfo,
    test::config::SupportedExtension,
    traits::{HasTask, PrioritisedRandom, Shared, User},
    utils::{get_extension_from_filename, get_timestamp_as_millis_as_string},
};
use rand::Rng;
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::{
    io::{self, AsyncWriteExt},
    sync::{mpsc, RwLock},
    task::JoinHandle,
    time::Instant,
};
use tokio_util::sync::CancellationToken;
use tracing_appender::non_blocking::WorkerGuard;

use self::{
    config::TestConfig,
    controller::{StopConditionData, TestController},
    user::{context::Context, EventsUserInfo, UserController, UserStatsCollection, UserStatus},
};

type SpawnUsersHandlesVector = Vec<JoinHandle<Vec<(JoinHandle<()>, u64)>>>;
pub struct Test {
    // TODO: No track of current users count!
    test_config: TestConfig,
    token: Arc<CancellationToken>,
    current_results_writer: Option<Writer>,
    results_history_writer: Option<Writer>,
    summary_writer: Option<Writer>,
    prometheus_current_metrics_writer: Option<Writer>,
    prometheus_metrics_history_writer: Option<TimeStapmedWriter>,
    total_users_spawned_arc_rwlock: Arc<RwLock<u64>>,
    all_results_arc_rwlock: Arc<RwLock<AllResults>>,
    user_stats_collection: UserStatsCollection,
    start_timestamp_arc_rwlock: Arc<RwLock<Instant>>,
    // on test drop, the worker guard will be dropped, which will stop the logging thread
    async_log_writer_worker_guard: Option<WorkerGuard>,
    prometheus_exporter_arc: Arc<PrometheusExporter>,
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
        let summary_writer = if let Some(summary_file) = &test_config.summary_file {
            match Writer::from_str(summary_file).await {
                Ok(writer) => Some(writer),
                Err(e) => {
                    tracing::error!("Failed to create writer for summary file: [{}]", e);
                    None
                }
            }
        } else {
            None
        };
        let prometheus_current_metrics_writer = if let Some(prometheus_current_metrics_file) =
            &test_config.prometheus_current_metrics_file
        {
            match Writer::from_str(prometheus_current_metrics_file).await {
                Ok(writer) => Some(writer),
                Err(e) => {
                    tracing::error!(
                        "Failed to create writer for prometheus current metrics file: [{}]",
                        e
                    );
                    None
                }
            }
        } else {
            None
        };
        let prometheus_metrics_history_writer = if let Some(prometheus_metrics_history_folder) =
            &test_config.prometheus_metrics_history_folder
        {
            match TimeStapmedWriter::from_str(
                prometheus_metrics_history_folder,
                String::from("metrics.prom"),
            )
            .await
            {
                Ok(writer) => Some(writer),
                Err(e) => {
                    tracing::error!(
                        "Failed to create writer for prometheus history metrics: [{}]",
                        e
                    );
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
            summary_writer,
            prometheus_current_metrics_writer,
            prometheus_metrics_history_writer,
            total_users_spawned_arc_rwlock: Arc::new(RwLock::new(0)),
            all_results_arc_rwlock: Arc::new(RwLock::new(AllResults::default())),
            user_stats_collection: UserStatsCollection::new(),
            start_timestamp_arc_rwlock: Arc::new(RwLock::new(Instant::now())),
            async_log_writer_worker_guard: None,
            prometheus_exporter_arc: Arc::new(PrometheusExporter::new()),
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
        // make sure we don't panic on an empty range
        if between.0 >= between.1 {
            tokio::time::sleep(Duration::from_secs(between.0)).await;
            return;
        }
        let sleep_time = rand::thread_rng().gen_range(between.0..between.1);
        tokio::time::sleep(Duration::from_secs(sleep_time)).await;
    }

    pub fn setup_logging(&mut self) {
        self.async_log_writer_worker_guard = setup_logging(
            self.test_config.log_level,
            self.test_config.log_to_stdout,
            self.test_config.log_file.clone(),
        );
    }

    // TODO: this method spawns users with a spawn rate, that depends on the user type and not the global spawn rate for all given users
    pub fn spawn_users<T, S>(
        &self,
        count: u64,
        starting_index: u64,
        results_tx: mpsc::UnboundedSender<MainMessage>,
        test_controller: Arc<TestController>,
        shared: S,
    ) -> JoinHandle<Vec<(JoinHandle<()>, u64)>>
    where
        T: HasTask + User + User<Shared = S>,
        S: Shared,
    {
        tracing::info!(
            "Spawning [{}][{}] users, starting at index [{}]",
            count,
            T::get_name(),
            starting_index
        );
        let tasks = Arc::new(T::get_async_tasks());
        if tasks.is_empty() {
            // TODO: total users spawned will always be logged since it will never reach total spawnbale users
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
                let id = i + starting_index;

                // these are the tokens for the test
                let test_token_for_user = token.clone();
                let test_spawn_token = token.clone();

                // create a user token for the UserController
                let user_token = Arc::new(CancellationToken::new());
                let user_controller = UserController::new(user_token.clone());
                let user_info = EventsUserInfo::new(id, T::get_name());
                let events_handler = EventsHandler::new(user_info, results_tx.clone());

                // create the data for the user
                let user_context = Context::new(
                    test_controller.clone(),
                    events_handler.clone(),
                    user_controller,
                );

                let tasks = tasks.clone();
                let shared = shared.clone();

                let handle = tokio::spawn(async move {
                    events_handler.add_user_spawned();
                    let mut user = T::new(&test_config, &user_context, shared).await;
                    user.on_start(&user_context).await;

                    loop {
                        // get a random task
                        if let Some(task) = tasks.get_prioritised_random() {
                            // call it, do some sleep
                            let task_call_and_sleep = async {
                                // this is the sleep time of a user
                                Test::sleep_between(between).await;

                                // this is the actual task
                                task.call(&mut user, &user_context).await;
                                user_context.get_events_handler().add_task_executed(
                                    EventsTaskInfo {
                                        name: task.name.clone(),
                                    },
                                );
                            };

                            tokio::select! {
                                _ = user_token.cancelled() => {
                                    tracing::info!("User [{}][{}] attempted suicide", T::get_name(), id);
                                    user_context.get_events_handler().add_user_self_stopped();
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
                    user.on_stop(&user_context).await;
                });
                handles.push((handle, id));
                users_spawned += 1;
                if users_spawned % users_per_sec == 0 {
                    tokio::select! {
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
        let prometheus_exporter_arc = self.prometheus_exporter_arc.clone();
        let addr = self.test_config.server_address;
        match addr {
            Some(addr) => {
                tracing::info!("Server listening on [{}]", addr);
                tokio::spawn(async move {
                    let server = Server::new(
                        test_controller,
                        all_results_arc_rwlock,
                        prometheus_exporter_arc,
                        addr,
                    );
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
        let prometheus_exporter_arc = self.prometheus_exporter_arc.clone();
        let start_timestamp_arc_rwlock = self.start_timestamp_arc_rwlock.clone();
        let test_config = self.test_config.clone();
        let current_results_writer = self.current_results_writer.clone();
        let results_history_writer = self.results_history_writer.clone();
        let prometheus_current_metrics_writer = self.prometheus_current_metrics_writer.clone();
        let prometheus_metrics_history_writer = self.prometheus_metrics_history_writer.clone();
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

                        if let Some(writer) = &prometheus_current_metrics_writer {
                            let prometheus_metrics_string = prometheus_exporter_arc.get_metrics();
                            match prometheus_metrics_string {
                                Ok(prometheus_metrics_string) => {
                                    match writer.write_all(prometheus_metrics_string.as_bytes()).await {
                                        Ok(_) => {}
                                        Err(e) => {
                                            tracing::error!("Error writing prometheus current metrics: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Error getting prometheus string: {}", e);
                                }
                            }
                        }

                        if let Some(writer) = &prometheus_metrics_history_writer {
                            let prometheus_metrics_string = prometheus_exporter_arc.get_metrics();
                            match prometheus_metrics_string {
                                Ok(prometheus_metrics_string) => {
                                    match writer.write_all(prometheus_metrics_string.as_bytes()).await {
                                        Ok(_) => {}
                                        Err(e) => {
                                            tracing::error!("Error writing prometheus metrics history: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Error getting prometheus string: {}", e);
                                }
                            }
                        }

                        // check stop condition and stop if needed
                        if let Some(stop_condition) = &test_config.stop_condition {
                            let stop_condition_data = StopConditionData::new(&all_results_gaurd, &elapsed_time);
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

    async fn block_on_reciever(&mut self, mut results_rx: mpsc::UnboundedReceiver<MainMessage>) {
        while let Some(msg) = results_rx.recv().await {
            match msg {
                MainMessage::ResultMessage(result_msg) => {
                    let mut all_results_gaurd = self.all_results_arc_rwlock.write().await;

                    match result_msg {
                        ResultMessage::Success(sucess_result_msg) => {
                            all_results_gaurd.add_success(
                                &sucess_result_msg.endpoint_type_name,
                                sucess_result_msg.response_time,
                            );

                            // updating user results
                            self.user_stats_collection.add_success(
                                &sucess_result_msg.user_info.id,
                                &sucess_result_msg.endpoint_type_name,
                                sucess_result_msg.response_time,
                            );

                            self.prometheus_exporter_arc.add_success(
                                RequestLabel {
                                    endpoint_type: sucess_result_msg.endpoint_type_name.r#type,
                                    endpoint_name: sucess_result_msg.endpoint_type_name.name,
                                    user_id: sucess_result_msg.user_info.id,
                                    user_name: sucess_result_msg.user_info.name,
                                },
                                sucess_result_msg.response_time,
                            );
                        }
                        ResultMessage::Failure(failure_result_msg) => {
                            all_results_gaurd.add_failure(&failure_result_msg.endpoint_type_name);

                            // updating user results
                            self.user_stats_collection.add_failure(
                                &failure_result_msg.user_info.id,
                                &failure_result_msg.endpoint_type_name,
                            );

                            self.prometheus_exporter_arc.add_failure(RequestLabel {
                                endpoint_type: failure_result_msg.endpoint_type_name.r#type,
                                endpoint_name: failure_result_msg.endpoint_type_name.name,
                                user_id: failure_result_msg.user_info.id,
                                user_name: failure_result_msg.user_info.name,
                            });
                        }
                        ResultMessage::Error(error_result_msg) => {
                            all_results_gaurd.add_error(
                                &error_result_msg.endpoint_type_name,
                                &error_result_msg.error,
                            );

                            // updating user results
                            self.user_stats_collection.add_error(
                                &error_result_msg.user_info.id,
                                &error_result_msg.endpoint_type_name,
                                &error_result_msg.error,
                            );

                            self.prometheus_exporter_arc.add_error(RequestLabel {
                                endpoint_type: error_result_msg.endpoint_type_name.r#type,
                                endpoint_name: error_result_msg.endpoint_type_name.name,
                                user_id: error_result_msg.user_info.id,
                                user_name: error_result_msg.user_info.name,
                            });
                        }
                    }
                }

                MainMessage::UserSpawned(user_spawned_msg) => {
                    let mut total_users_spawned_gaurd =
                        self.total_users_spawned_arc_rwlock.write().await;
                    *total_users_spawned_gaurd += 1;

                    self.user_stats_collection.insert_user(
                        user_spawned_msg.user_info.id,
                        user_spawned_msg.user_info.name.clone(),
                    );

                    self.prometheus_exporter_arc.add_user(UserLabel {
                        user_name: user_spawned_msg.user_info.name,
                    });
                }

                // tasks with suicide or panic are not included
                MainMessage::TaskExecuted(user_fired_task_msg) => {
                    self.user_stats_collection
                        .increment_total_tasks(&user_fired_task_msg.user_info.id);

                    self.prometheus_exporter_arc.add_task(TaskLabel {
                        user_id: user_fired_task_msg.user_info.id,
                        user_name: user_fired_task_msg.user_info.name,
                        task_name: user_fired_task_msg.task_info.name,
                    });
                }

                MainMessage::UserSelfStopped(user_self_stopped_msg) => {
                    self.user_stats_collection.set_user_status(
                        &user_self_stopped_msg.user_info.id,
                        UserStatus::Cancelled,
                    );

                    self.prometheus_exporter_arc.remove_user(UserLabel {
                        user_name: user_self_stopped_msg.user_info.name,
                    });
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
        &mut self,
        results_rx: mpsc::UnboundedReceiver<MainMessage>,
        spawn_users_handles_vec: SpawnUsersHandlesVector,
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
                    for (handle, id) in handles {
                        let status = match handle.await {
                            Ok(_) => UserStatus::Finished,
                            Err(e) => {
                                if e.is_panic() {
                                    UserStatus::Panicked
                                } else {
                                    UserStatus::Unknown
                                }
                                // if e.is_cancelled() {
                                //     UserStatus::Unknown
                                // }
                            }
                        };
                        self.user_stats_collection.set_user_status(&id, status);
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

        self.write_summary_to_file().await;

        tracing::info!("Test finished");
    }

    async fn write_summary_to_file(&mut self) {
        if let Some(summary_writer) = &self.summary_writer {
            tracing::info!("Writing summary to file");
            let elapsed_time =
                Test::calculate_elapsed_time(&*self.start_timestamp_arc_rwlock.read().await);
            self.user_stats_collection
                .calculate_per_second(&elapsed_time);

            let extension = SupportedExtension::from_str(
                get_extension_from_filename(summary_writer.get_path()).unwrap_or(""),
            )
            .unwrap_or(SupportedExtension::Json);
            let summary_string = match extension {
                SupportedExtension::Yaml => self.user_stats_collection.yaml_string(),
                SupportedExtension::Json => self.user_stats_collection.json_string(),
            };
            match summary_string {
                Ok(summary_string) => {
                    if let Err(e) = summary_writer.write_all(summary_string.as_bytes()).await {
                        tracing::error!("Error writing summary to file: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Error serializing summary: {}", e);
                }
            }
        }
    }
}

impl Drop for Test {
    // test is not clone so this is fine to stop the test on drop
    fn drop(&mut self) {
        self.token.cancel();
    }
}
