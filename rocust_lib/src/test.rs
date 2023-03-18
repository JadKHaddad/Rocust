pub mod config;
pub(crate) mod controller;
pub mod user;

use crate::{
    events::EventsHandler,
    fs::{timestamped_writer::TimeStapmedWriter, writer::Writer},
    messages::{MainMessage, ResultMessage},
    prometheus_exporter::{PrometheusExporter, RequestLabel, TaskLabel, UserCountLabel, UserLabel},
    results::AllResults,
    server::Server,
    tasks::EventsTaskInfo,
    test::config::SupportedExtension,
    traits::{HasTask, PrioritisedRandom, Shared, User},
    utils,
};
use rand::Rng;
use std::{path::Path, str::FromStr, sync::Arc, time::Duration};
use tokio::{
    io::{self, AsyncWriteExt},
    sync::{mpsc, RwLock},
    task::JoinHandle,
    time::Instant,
};
use tokio_util::sync::CancellationToken;

use self::{
    config::TestConfig,
    controller::TestController,
    user::{context::Context, EventsUserInfo, UserController, UserStatsCollection, UserStatus},
};

type SpawnUsersHandlesVector = Vec<JoinHandle<Vec<(JoinHandle<()>, u64)>>>;
pub struct Test {
    test_config: TestConfig,
    token: Arc<CancellationToken>,
    writers: Writers,
    total_users_spawned_arc_rwlock: Arc<RwLock<u64>>,
    all_results_arc_rwlock: Arc<RwLock<AllResults>>,
    user_stats_collection: UserStatsCollection,
    start_timestamp_arc_rwlock: Arc<RwLock<Instant>>,
    prometheus_exporter_arc: Arc<PrometheusExporter>,
}

#[derive(Clone)]
pub(crate) struct Writers {
    current_results_writer: Option<Writer>,
    results_history_writer: Option<Writer>,
    summary_writer: Option<Writer>,
    prometheus_current_metrics_writer: Option<Writer>,
    prometheus_metrics_history_writer: Option<TimeStapmedWriter>,
}

impl Test {
    pub async fn new(test_config: TestConfig) -> Self {
        let writers = Writers::new(&test_config).await;
        Self {
            test_config,
            token: Arc::new(CancellationToken::new()),
            writers,
            total_users_spawned_arc_rwlock: Arc::new(RwLock::new(0)),
            all_results_arc_rwlock: Arc::new(RwLock::new(AllResults::default())),
            user_stats_collection: UserStatsCollection::new(),
            start_timestamp_arc_rwlock: Arc::new(RwLock::new(Instant::now())),
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

    fn update_stats_on_update_interval(elapsed_time: &Duration, all_results: &mut AllResults) {
        all_results.calculate_on_update_interval(elapsed_time);
    }

    async fn print_stats_to_stdout(
        precision: usize,
        print_to_stdout: bool,
        all_results: &AllResults,
    ) {
        if print_to_stdout {
            let table_string = all_results.table_string(precision);
            let mut stdout = io::stdout();
            let _ = stdout.write_all(table_string.as_bytes()).await;
        }
    }

    fn get_supported_summary_extension(path: &Path) -> SupportedExtension {
        let extension =
            SupportedExtension::from_str(utils::get_extension_from_filename(path).unwrap_or(""))
                .unwrap_or(SupportedExtension::Json);

        extension
    }

    async fn sleep_between(between: (u64, u64)) {
        // make sure we don't panic on an empty range
        if between.0 >= between.1 {
            tokio::time::sleep(Duration::from_secs(between.0)).await;
            return;
        }
        let sleep_time = rand::thread_rng().gen_range(between.0..=between.1);
        tokio::time::sleep(Duration::from_secs(sleep_time)).await;
    }

    // TODO: this method spawns users with a spawn rate, that depends on the user type and not the global spawn rate for all given users
    pub fn spawn_users<T, S>(
        &self,
        count: u64,
        starting_index: u64,
        results_tx: mpsc::Sender<MainMessage>,
        test_controller: Arc<TestController>,
        shared: S,
    ) -> JoinHandle<Vec<(JoinHandle<()>, u64)>>
    where
        T: HasTask + User + User<Shared = S>,
        S: Shared,
    {
        tracing::info!(
            user_name = T::get_name(),
            user_count = count,
            %starting_index,
            "Spawning users",
        );
        let tasks = Arc::new(T::get_async_tasks());
        if tasks.is_empty() {
            // TODO: total users spawned will always be logged since it will never reach total spawnbale users
            tracing::warn!(
                user_name = T::get_name(),
                "User has no tasks. Will not be spawned"
            );
            return tokio::spawn(async move { vec![] }); // just to avoid an infinite loop
        }
        let between = T::get_between();

        let token = self.token.clone();

        let test_config = self.test_config.clone();
        let users_per_sec = test_config.users_per_sec;
        tokio::spawn(async move {
            let mut supervisors = vec![];
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
                let supervisor_events_handler = events_handler.clone();

                // create the data for the user
                let user_context = Context::new(
                    test_controller.clone(),
                    events_handler.clone(),
                    user_controller,
                );

                let tasks = tasks.clone();
                let shared = shared.clone();
                let supervisor = tokio::spawn(async move {
                    let handle = tokio::spawn(async move {
                        events_handler.add_user_spawned().await;
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
                                };

                                tokio::select! {
                                    _ = user_token.cancelled() => {
                                        user.on_stop(&user_context).await;
                                        return UserStatus::Cancelled;
                                    }
                                    _ = test_token_for_user.cancelled() => {
                                        user.on_stop(&user_context).await;
                                        return UserStatus::Finished;
                                    }
                                    _ = task_call_and_sleep => {
                                        events_handler
                                        .add_task_executed(EventsTaskInfo { name: task.name }).await;
                                    }
                                }
                            }
                        }
                    });

                    match handle.await {
                        Ok(status) => match status {
                            UserStatus::Finished => {
                                supervisor_events_handler.add_user_finished().await;
                            }
                            UserStatus::Cancelled => {
                                supervisor_events_handler.add_user_self_stopped().await;
                            }
                            _ => {
                                // well obviously unreachable
                            }
                        },
                        Err(e) => {
                            if e.is_panic() {
                                supervisor_events_handler
                                    .add_user_panicked(e.to_string())
                                    .await;
                            } else {
                                // very unlikely
                                supervisor_events_handler.add_user_unknown_status().await;
                            }
                        } // at this point we can decide what to do with the user, maybe restart it?
                    }
                });

                supervisors.push((supervisor, id));
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
            supervisors
        })
    }

    fn start_timer(&self) -> JoinHandle<()> {
        let token = self.token.clone();
        tracing::info!(runtime = self.test_config.runtime, "Starting timer");
        match self.test_config.runtime {
            Some(runtime) => {
                tokio::spawn(async move {
                    tokio::select! {
                        // this could be ctrl+c or any other signal
                        _ = token.cancelled() => {
                            tracing::debug!("Signal received");
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
                tokio::spawn(async move {
                    // this could be ctrl+c or any other signal
                    token.cancelled().await;
                    tracing::debug!("Signal received");
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
                tracing::info!(address = ?addr, "Starting server");
                tokio::spawn(async move {
                    let server = Server::new(
                        test_controller,
                        all_results_arc_rwlock,
                        prometheus_exporter_arc,
                        addr,
                    );
                    // no tokio::select! here because axum is running with graceful shutdown
                    let res = server.run().await;
                    if let Err(error) = res {
                        tracing::error!(%error, "Server error");
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
        let writers = self.writers.clone();
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
                            tracing::info!(total_users_spawned=*total_users_spawned_gaurd,total_spawnable_users=total_spawnable_user_count,  "Spawning users");
                            if *total_users_spawned_gaurd == total_spawnable_user_count {
                                tracing::info!(total_spawnable_users=total_spawnable_user_count, "All users spawned");
                                print_total_spawned_users = false;
                            }
                        }

                        let mut all_results_gaurd = all_results_arc_rwlock.write().await;

                        let elapsed_time = Test::calculate_elapsed_time(&*start_timestamp_arc_rwlock.read().await);

                        Test::update_stats_on_update_interval(&elapsed_time, &mut *all_results_gaurd);

                        Test::print_stats_to_stdout(test_config.precision, test_config.print_to_stdout, &*all_results_gaurd).await;

                        writers.write_on_update_interval(&*all_results_gaurd, &*prometheus_exporter_arc).await;
                    }
                }
            }
        })
    }

    async fn block_on_reciever(&mut self, mut results_rx: mpsc::Receiver<MainMessage>) {
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
                    tracing::trace!(
                        user_name = &user_spawned_msg.user_info.name,
                        user_id = &user_spawned_msg.user_info.id,
                        "User spawned"
                    );

                    let mut total_users_spawned_gaurd =
                        self.total_users_spawned_arc_rwlock.write().await;
                    *total_users_spawned_gaurd += 1;

                    self.user_stats_collection.insert_user(
                        user_spawned_msg.user_info.id,
                        user_spawned_msg.user_info.name,
                    );

                    self.prometheus_exporter_arc.add_user(UserCountLabel {
                        user_name: user_spawned_msg.user_info.name,
                    });
                }

                // tasks with suicide or panic are not included
                MainMessage::TaskExecuted(user_fired_task_msg) => {
                    tracing::trace!(
                        user_name = &user_fired_task_msg.user_info.name,
                        user_id = &user_fired_task_msg.user_info.id,
                        task_name = &user_fired_task_msg.task_info.name,
                        "User excuted a task"
                    );

                    self.user_stats_collection
                        .increment_total_tasks(&user_fired_task_msg.user_info.id);

                    self.prometheus_exporter_arc.add_task(TaskLabel {
                        user_id: user_fired_task_msg.user_info.id,
                        user_name: user_fired_task_msg.user_info.name,
                        task_name: user_fired_task_msg.task_info.name,
                    });
                }

                MainMessage::UserSelfStopped(user_self_stopped_msg) => {
                    tracing::info!(
                        user_name = &user_self_stopped_msg.user_info.name,
                        user_id = &user_self_stopped_msg.user_info.id,
                        "User attempted suicide",
                    );

                    self.user_stats_collection.set_user_status(
                        &user_self_stopped_msg.user_info.id,
                        UserStatus::Cancelled,
                    );

                    self.prometheus_exporter_arc.remove_user(UserCountLabel {
                        user_name: user_self_stopped_msg.user_info.name,
                    });

                    self.prometheus_exporter_arc.add_suicide(UserLabel {
                        user_id: user_self_stopped_msg.user_info.id,
                        user_name: user_self_stopped_msg.user_info.name,
                    });
                }

                MainMessage::UserFinished(user_finished_msg) => {
                    self.user_stats_collection
                        .set_user_status(&user_finished_msg.user_info.id, UserStatus::Finished);
                }

                MainMessage::UserPanicked(user_panicked_msg) => {
                    tracing::warn!(
                        user_name = &user_panicked_msg.user_info.name,
                        user_id = &user_panicked_msg.user_info.id,
                        "User panicked!"
                    );

                    self.user_stats_collection
                        .set_user_status(&user_panicked_msg.user_info.id, UserStatus::Panicked);

                    self.prometheus_exporter_arc.remove_user(UserCountLabel {
                        user_name: user_panicked_msg.user_info.name,
                    });

                    self.prometheus_exporter_arc.add_panic(UserLabel {
                        user_id: user_panicked_msg.user_info.id,
                        user_name: user_panicked_msg.user_info.name,
                    });
                }

                MainMessage::UserUnknownStatus(user_unknown_status_msg) => {
                    tracing::warn!(
                        user_name = &user_unknown_status_msg.user_info.name,
                        user_id = &user_unknown_status_msg.user_info.id,
                        "User has unknown status!. Supervisor failed to get user status",
                    );

                    self.user_stats_collection.set_user_status(
                        &user_unknown_status_msg.user_info.id,
                        UserStatus::Unknown,
                    );

                    self.prometheus_exporter_arc.remove_user(UserCountLabel {
                        user_name: user_unknown_status_msg.user_info.name,
                    });
                }
            }
        }
    }

    pub async fn before_spawn_users(
        &self,
    ) -> (mpsc::Sender<MainMessage>, mpsc::Receiver<MainMessage>) {
        // set timestamp
        *self.start_timestamp_arc_rwlock.write().await = Instant::now();
        mpsc::channel(100)
    }

    pub async fn after_spawn_users(
        &mut self,
        results_rx: mpsc::Receiver<MainMessage>,
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
        tracing::debug!("Main reciever dropped");

        // this will cancel the timer and background tasks if the only given user has no tasks so it will finish immediately thus causing the reciever to drop
        self.token.cancel();

        // wait for all users to finish
        for spawn_users_handles in spawn_users_handles_vec {
            match spawn_users_handles.await {
                Ok(supervisors) => {
                    for (supervisor, id) in supervisors {
                        if let Err(error) = supervisor.await {
                            self.user_stats_collection
                                .set_user_status(&id, UserStatus::Unknown);
                            tracing::error!(%error, user_id=id, "Error joining supervisor for user");
                        };
                    }
                }
                Err(error) => {
                    tracing::error!(%error, "Error joining supervisors");
                }
            }
        }

        match server_handle.await {
            Ok(_) => {
                tracing::debug!("Server joined");
            }
            Err(error) => {
                tracing::error!(%error, "Error joining server");
            }
        }

        match background_tasks_handle.await {
            Ok(_) => {
                tracing::debug!("Background tasks joined");
            }
            Err(error) => {
                tracing::error!(%error, "Error joining background tasks");
            }
        }

        match timer_handle.await {
            Ok(_) => {
                tracing::debug!("Timer joined");
            }
            Err(error) => {
                tracing::error!(%error, "Error joining timer");
            }
        }
        let elapsed_time =
            Test::calculate_elapsed_time(&*self.start_timestamp_arc_rwlock.read().await);

        self.update_summary_and_write_to_file(&elapsed_time).await;

        let mut all_results_gaurd = self.all_results_arc_rwlock.write().await;

        Test::update_stats_on_update_interval(&elapsed_time, &mut *all_results_gaurd);

        self.writers
            .write_on_update_interval(&*all_results_gaurd, &*self.prometheus_exporter_arc)
            .await;

        tracing::info!("Test terminated");
    }

    fn get_summary_string_from_extension(
        &self,
        extension: SupportedExtension,
    ) -> Result<String, user::UserStatsCollectionError> {
        match extension {
            SupportedExtension::Yaml => self.user_stats_collection.yaml_string(),
            SupportedExtension::Json => self.user_stats_collection.json_string(),
        }
    }

    fn get_summary_string_from_path(
        &self,
        path: &Path,
    ) -> Result<String, user::UserStatsCollectionError> {
        let extension = Test::get_supported_summary_extension(path);
        self.get_summary_string_from_extension(extension)
    }

    async fn update_summary_and_write_to_file(&mut self, elapsed_time: &Duration) {
        if let Some(summary_writer) = &self.writers.summary_writer {
            self.user_stats_collection
                .calculate_on_update_interval(&elapsed_time);

            let summary_string = self.get_summary_string_from_path(summary_writer.get_path());

            match summary_string {
                Ok(summary_string) => {
                    if let Err(error) = summary_writer.write_all(summary_string.as_bytes()).await {
                        tracing::error!(%error, "Error writing summary to file");
                    }
                }
                Err(error) => {
                    tracing::error!(%error, "Error serializing summar");
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

impl Writers {
    pub async fn new(test_config: &TestConfig) -> Self {
        let current_results_writer =
            if let Some(current_results_file) = &test_config.current_results_file {
                match Writer::from_str(current_results_file).await {
                    Ok(writer) => Some(writer),
                    Err(error) => {
                        tracing::error!(%error, "Failed to create writer for current results file");
                        None
                    }
                }
            } else {
                None
            };
        let results_history_writer =
            if let Some(results_history_file) = &test_config.results_history_file {
                match Writer::from_str(results_history_file).await {
                    Ok(writer) => {
                        // write header
                        let header = AllResults::history_header_csv_string();
                        match header {
                            Ok(header) => match writer.write_all(header.as_bytes()).await {
                                Ok(_) => Some(writer),
                                Err(error) => {
                                    tracing::error!(
                                        %error,
                                        "Failed to write header to results history file"
                                    );
                                    None
                                }
                            },
                            Err(error) => {
                                tracing::error!(
                                    %error,
                                    "Failed to create header for results history file",
                                );
                                None
                            }
                        }
                    }
                    Err(error) => {
                        tracing::error!(%error, "Failed to create writer for results history file");
                        None
                    }
                }
            } else {
                None
            };
        let summary_writer = if let Some(summary_file) = &test_config.summary_file {
            match Writer::from_str(summary_file).await {
                Ok(writer) => Some(writer),
                Err(error) => {
                    tracing::error!(%error, "Failed to create writer for summary file");
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
                Err(error) => {
                    tracing::error!(
                        %error,
                        "Failed to create writer for prometheus current metrics file"
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
                Err(error) => {
                    tracing::error!(
                        %error,
                        "Failed to create writer for prometheus history metrics"
                    );
                    None
                }
            }
        } else {
            None
        };
        Self {
            current_results_writer,
            results_history_writer,
            summary_writer,
            prometheus_current_metrics_writer,
            prometheus_metrics_history_writer,
        }
    }

    async fn write_current_results(&self, all_results: &AllResults) {
        if let Some(writer) = &self.current_results_writer {
            let csv_string = all_results.current_results_csv_string();
            match csv_string {
                Ok(csv_string) => {
                    if let Err(error) = writer.write_all(csv_string.as_bytes()).await {
                        tracing::error!(%error, "Error writing to csv");
                    }
                }
                Err(error) => {
                    tracing::error!(%error, "Error getting csv string");
                }
            }
        }
    }

    async fn write_results_history(&self, all_results: &AllResults) {
        if let Some(writer) = &self.results_history_writer {
            match utils::get_timestamp_as_millis_as_string() {
                Ok(timestamp) => {
                    let csv_string = all_results
                        .current_aggrigated_results_with_timestamp_csv_string(&timestamp);
                    match csv_string {
                        Ok(csv_string) => {
                            if let Err(error) = writer.append_all(csv_string.as_bytes()).await {
                                tracing::error!(%error, "Error writing to csv");
                            }
                        }
                        Err(error) => {
                            tracing::error!(%error, "Error getting csv string");
                        }
                    }
                }
                Err(error) => {
                    tracing::error!(%error, "Error getting timestamp");
                }
            }
        }
    }

    async fn write_prometheus_current_metrics(&self, prometheus_exporter: &PrometheusExporter) {
        if let Some(writer) = &self.prometheus_current_metrics_writer {
            let prometheus_metrics_string = prometheus_exporter.get_metrics();
            match prometheus_metrics_string {
                Ok(prometheus_metrics_string) => {
                    if let Err(error) = writer.write_all(prometheus_metrics_string.as_bytes()).await
                    {
                        tracing::error!(%error, "Error writing prometheus current metrics");
                    }
                }
                Err(error) => {
                    tracing::error!(%error, "Error getting prometheus string");
                }
            }
        }
    }

    async fn write_prometheus_metrics_history(&self, prometheus_exporter: &PrometheusExporter) {
        if let Some(writer) = &self.prometheus_metrics_history_writer {
            let prometheus_metrics_string = prometheus_exporter.get_metrics();
            match prometheus_metrics_string {
                Ok(prometheus_metrics_string) => {
                    if let Err(error) = writer.write_all(prometheus_metrics_string.as_bytes()).await
                    {
                        tracing::error!(%error, "Error writing prometheus metrics history");
                    }
                }
                Err(error) => {
                    tracing::error!(%error, "Error getting prometheus string");
                }
            }
        }
    }

    async fn write_on_update_interval(
        &self,
        all_results: &AllResults,
        prometheus_exporter: &PrometheusExporter,
    ) {
        self.write_current_results(all_results).await;
        self.write_results_history(all_results).await;
        self.write_prometheus_current_metrics(prometheus_exporter)
            .await;
        self.write_prometheus_metrics_history(prometheus_exporter)
            .await;
    }
}
