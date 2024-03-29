pub mod config;
pub(crate) mod controller;
pub mod spawn_coordinator;
pub mod user;
mod writers;

use crate::{
    messages::{MainMessage, ResultMessage},
    prometheus_exporter::{PrometheusExporter, RequestLabel, TaskLabel, UserCountLabel, UserLabel},
    results::AllResults,
    server::Server,
    test::config::SupportedExtension,
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
    spawn_coordinator::SpawnCoordinator,
    user::{UserStatsCollection, UserStatus},
    writers::Writers,
};

type SpawnUsersHandlesVector = Vec<JoinHandle<Vec<(JoinHandle<()>, u64)>>>;
pub struct Test {
    test_config: TestConfig,
    token: CancellationToken,
    writers: Writers,
    total_users_spawned_arc_rwlock: Arc<RwLock<u64>>,
    all_results_arc_rwlock: Arc<RwLock<AllResults>>,
    user_stats_collection: UserStatsCollection,
    start_timestamp_arc_rwlock: Arc<RwLock<Instant>>,
    prometheus_exporter_arc: Arc<PrometheusExporter>,
}

impl Test {
    #[must_use]
    pub async fn new(test_config: TestConfig) -> Self {
        let writers = Writers::new(&test_config).await;
        Self {
            test_config,
            token: CancellationToken::new(),
            writers,
            total_users_spawned_arc_rwlock: Arc::new(RwLock::new(0)),
            all_results_arc_rwlock: Arc::new(RwLock::new(AllResults::default())),
            user_stats_collection: UserStatsCollection::new(),
            start_timestamp_arc_rwlock: Arc::new(RwLock::new(Instant::now())),
            prometheus_exporter_arc: Arc::new(PrometheusExporter::new()),
        }
    }

    pub fn clone_token(&self) -> CancellationToken {
        self.token.clone()
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

                        all_results_gaurd.calculate_on_update_interval(&elapsed_time);

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
                    self.on_result_message(result_msg).await;
                }

                MainMessage::UserSpawned(user_spawned_msg) => {
                    self.on_user_spawned_message(user_spawned_msg).await;
                }

                // tasks with suicide or panic are not included
                MainMessage::TaskExecuted(user_fired_task_msg) => {
                    self.on_task_excuted_message(user_fired_task_msg);
                }

                MainMessage::UserSelfStopped(user_self_stopped_msg) => {
                    self.on_user_self_stopped_message(user_self_stopped_msg);
                }

                MainMessage::UserFinished(user_finished_msg) => {
                    self.on_user_finished_message(user_finished_msg);
                }

                MainMessage::UserPanicked(user_panicked_msg) => {
                    self.on_user_panicked_message(user_panicked_msg);
                }

                MainMessage::UserUnknownStatus(user_unknown_status_msg) => {
                    self.on_user_unknown_status_message(user_unknown_status_msg);
                }
            }
        }
        tracing::debug!("Main reciever dropped");
    }

    // will not be refactored because of borrow checker issues :->
    async fn on_result_message(&mut self, result_msg: ResultMessage) {
        let mut all_results_gaurd = self.all_results_arc_rwlock.write().await;

        match result_msg {
            ResultMessage::Success(sucess_result_msg) => {
                all_results_gaurd.add_success(
                    &sucess_result_msg.endpoint_type_name,
                    sucess_result_msg.response_time,
                );

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

    #[inline]
    async fn on_user_spawned_message(
        &mut self,
        user_spawned_msg: crate::messages::UserSpawnedMessage,
    ) {
        tracing::trace!(
            user_name = &user_spawned_msg.user_info.name,
            user_id = &user_spawned_msg.user_info.id,
            "User spawned"
        );

        let mut total_users_spawned_gaurd = self.total_users_spawned_arc_rwlock.write().await;
        *total_users_spawned_gaurd += 1;

        self.user_stats_collection.insert_user(
            user_spawned_msg.user_info.id,
            user_spawned_msg.user_info.name,
        );

        self.prometheus_exporter_arc.add_user(UserCountLabel {
            user_name: user_spawned_msg.user_info.name,
        });
    }

    #[inline]
    fn on_user_self_stopped_message(
        &mut self,
        user_self_stopped_msg: crate::messages::UserSelfStoppedMessage,
    ) {
        tracing::info!(
            user_name = &user_self_stopped_msg.user_info.name,
            user_id = &user_self_stopped_msg.user_info.id,
            "User attempted suicide",
        );

        self.user_stats_collection
            .set_user_status(&user_self_stopped_msg.user_info.id, UserStatus::Cancelled);

        self.prometheus_exporter_arc.remove_user(UserCountLabel {
            user_name: user_self_stopped_msg.user_info.name,
        });

        self.prometheus_exporter_arc.add_suicide(UserLabel {
            user_id: user_self_stopped_msg.user_info.id,
            user_name: user_self_stopped_msg.user_info.name,
        });
    }

    #[inline]
    fn on_task_excuted_message(
        &mut self,
        user_fired_task_msg: crate::messages::TaskExecutedMessage,
    ) {
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

    #[inline]
    fn on_user_finished_message(
        &mut self,
        user_finished_msg: crate::messages::UserFinishedMessage,
    ) {
        self.user_stats_collection
            .set_user_status(&user_finished_msg.user_info.id, UserStatus::Finished);
    }

    #[inline]
    fn on_user_panicked_message(
        &mut self,
        user_panicked_msg: crate::messages::UserPanickedMessage,
    ) {
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

    #[inline]
    fn on_user_unknown_status_message(
        &mut self,
        user_unknown_status_msg: crate::messages::UserUnknownStatusMessage,
    ) {
        tracing::warn!(
            user_name = &user_unknown_status_msg.user_info.name,
            user_id = &user_unknown_status_msg.user_info.id,
            "User has unknown status!. Supervisor failed to get user status",
        );

        self.user_stats_collection
            .set_user_status(&user_unknown_status_msg.user_info.id, UserStatus::Unknown);

        self.prometheus_exporter_arc.remove_user(UserCountLabel {
            user_name: user_unknown_status_msg.user_info.name,
        });
    }

    pub async fn sink(
        &mut self,
        results_rx: mpsc::Receiver<MainMessage>,
        spawn_coordinator: SpawnCoordinator,
        spawn_users_handles_vec: SpawnUsersHandlesVector,
        total_spawnable_user_count: u64,
    ) {
        *self.start_timestamp_arc_rwlock.write().await = Instant::now();

        let spawn_coordinator_handle = spawn_coordinator.run();
        let server_handle = self.strat_server();
        let timer_handle = self.start_timer();
        //(calculating stats, printing stats, managing files)
        let background_tasks_handle = self.start_background_tasks(total_spawnable_user_count);

        self.block_on_reciever(results_rx).await;

        self.join_tasks(
            spawn_coordinator_handle,
            spawn_users_handles_vec,
            server_handle,
            background_tasks_handle,
            timer_handle,
        )
        .await;

        let elapsed_time =
            Test::calculate_elapsed_time(&*self.start_timestamp_arc_rwlock.read().await);

        self.update_summary_and_write_to_file(&elapsed_time).await;

        let mut all_results_gaurd = self.all_results_arc_rwlock.write().await;

        all_results_gaurd.calculate_on_update_interval(&elapsed_time);

        self.writers
            .write_on_update_interval(&*all_results_gaurd, &*self.prometheus_exporter_arc)
            .await;

        tracing::info!("Test terminated");
    }

    async fn join_tasks(
        &mut self,
        spawn_coordinator_handle: JoinHandle<()>,
        spawn_users_handles_vec: Vec<JoinHandle<Vec<(JoinHandle<()>, u64)>>>,
        server_handle: JoinHandle<()>,
        background_tasks_handle: JoinHandle<()>,
        timer_handle: JoinHandle<()>,
    ) {
        match spawn_coordinator_handle.await {
            Ok(_) => {
                tracing::debug!("Spawn coordinator joined");
            }
            Err(error) => {
                tracing::error!(%error, "Error joining spawn coordinator");
            }
        }

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
        if let Some(summary_writer) = &self.writers.get_summary_writer() {
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
