use super::controller::TestController;
use crate::{
    events::EventsHandler,
    messages::MainMessage,
    tasks::{AsyncTask, EventsTaskInfo},
    test::user::{EventsUserInfo, UserController, UserStatus},
    traits::{HasTask, PrioritisedRandom},
    utils::shift_vec,
    Context, Shared, Test, TestConfig, User,
};
use std::sync::Arc;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

pub struct UserSpawnController {
    user_name: &'static str,
    limit: u64,
    total_spawned: u64,
    spawn_tx: UnboundedSender<u64>,
}

impl UserSpawnController {
    pub fn new(user_name: &'static str, limit: u64, spawn_tx: UnboundedSender<u64>) -> Self {
        Self {
            user_name,
            limit,
            total_spawned: 0,
            spawn_tx,
        }
    }

    fn spawn_count(&mut self, count: u64) {
        let mut to_spawn_count = count;

        if self.total_spawned + count >= self.limit {
            to_spawn_count = self.limit - self.total_spawned;
            self.total_spawned = self.limit;
        } else {
            self.total_spawned += count;
        }

        let _ = self.spawn_tx.send(to_spawn_count);

        tracing::debug!(
            count = to_spawn_count,
            user_name = self.user_name,
            total_users_spawned = self.total_spawned,
            total_spawnable_users = self.limit,
            "Spawning users"
        );
    }

    fn is_complete(&self) -> bool {
        self.total_spawned >= self.limit
    }
}

pub struct SpawnCoordinator {
    users_per_sec: u64,
    user_spawn_controllers: Vec<UserSpawnController>,
    token: CancellationToken,
}

impl SpawnCoordinator {
    pub fn new(
        users_per_sec: u64,
        user_spawn_controllers: Vec<UserSpawnController>,
        token: CancellationToken,
    ) -> Self {
        Self {
            users_per_sec,
            user_spawn_controllers,
            token,
        }
    }

    async fn spawn(&mut self) {
        let len = self.user_spawn_controllers.len() as u64;
        loop {
            if self.user_spawn_controllers.is_empty() {
                break;
            }

            if self.users_per_sec < len {
                let count = 1;
                for i in 0..self.users_per_sec {
                    self.user_spawn_controllers[i as usize].spawn_count(count);
                }
                shift_vec(&mut self.user_spawn_controllers);
            } else if self.users_per_sec == len {
                let count = 1;
                for user_spawn_controller in &mut self.user_spawn_controllers {
                    user_spawn_controller.spawn_count(count);
                }
            } else if self.users_per_sec % len == 0 {
                let count = self.users_per_sec / len;
                for user_spawn_controller in &mut self.user_spawn_controllers {
                    user_spawn_controller.spawn_count(count);
                }
            } else {
                let count = self.users_per_sec / len;
                for i in 0..self.users_per_sec % len {
                    self.user_spawn_controllers[i as usize].spawn_count(count + 1);
                }
                for i in self.users_per_sec % len..len {
                    self.user_spawn_controllers[i as usize].spawn_count(count);
                }
                shift_vec(&mut self.user_spawn_controllers);
            }

            self.user_spawn_controllers
                .retain(|user_spawn_controller| !user_spawn_controller.is_complete());

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    pub fn run(mut self) -> JoinHandle<()> {
        let token = self.token.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = token.cancelled() => {
                    tracing::debug!("Spawn coordinator cancelled");
                }
                _ = self.spawn() => {
                    tracing::debug!("Spawn coordinator finished");
                }
            }
        })
    }
}

pub struct Spawner<T, S>
where
    T: HasTask + User + User<Shared = S>,
    S: Shared,
{
    user_name: &'static str,
    tasks: Arc<Vec<AsyncTask<T>>>,
    between: (u64, u64),
    user_count: u64,
    token: CancellationToken,
    test_config: TestConfig,
    test_controller: Arc<TestController>,
    results_tx: mpsc::Sender<MainMessage>,
    starting_index: u64,
    shared: S,
    spawn_coordinator_rx: UnboundedReceiver<u64>,
}

impl<T, S> Spawner<T, S>
where
    T: HasTask + User + User<Shared = S>,
    S: Shared,
{
    pub fn new(
        user_count: u64,
        token: CancellationToken,
        test_config: TestConfig,
        test_controller: Arc<TestController>,
        results_tx: mpsc::Sender<MainMessage>,
        starting_index: u64,
        shared: S,
        spawn_coordinator_rx: UnboundedReceiver<u64>,
    ) -> Self {
        Self {
            user_name: T::get_name(),
            tasks: Arc::new(T::get_async_tasks()),
            between: T::get_between(),
            user_count,
            token,
            test_config,
            test_controller,
            results_tx,
            starting_index,
            shared,
            spawn_coordinator_rx,
        }
    }
    pub fn run(mut self) -> JoinHandle<Vec<(JoinHandle<()>, u64)>> {
        tracing::info!(
            user_name = self.user_name,
            user_count = self.user_count,
            starting_index = self.starting_index,
            "Spawning users",
        );

        let tasks = self.tasks.clone();
        let token = self.token.clone();
        let test_config = self.test_config.clone();
        let results_tx = self.results_tx.clone();
        let test_controller = self.test_controller.clone();

        tokio::spawn(async move {
            let mut supervisors = vec![];
            let mut current_index = self.starting_index;
            while let Some(spawn_count) = self.spawn_coordinator_rx.recv().await {
                for _ in 0..spawn_count {
                    let id = current_index;

                    let test_config = test_config.clone();

                    // these are the tokens for the test
                    let test_token_for_user = token.clone();

                    // create a user token for the UserController
                    let user_token = CancellationToken::new();
                    let user_controller = UserController::new(user_token.clone());
                    let user_info = EventsUserInfo::new(id, self.user_name);
                    let events_handler = EventsHandler::new(user_info, results_tx.clone());
                    let supervisor_events_handler = events_handler.clone();

                    // create the data for the user
                    let user_context = Context::new(
                        test_controller.clone(),
                        events_handler.clone(),
                        user_controller,
                    );

                    let tasks = tasks.clone();
                    let shared = self.shared.clone();
                    let supervisor = tokio::spawn(async move {
                        let handle = tokio::spawn(async move {
                            events_handler.add_user_spawned().await;
                            let mut user = T::new(&test_config, &user_context, shared).await;
                            user.on_start(&user_context).await;

                            if tasks.is_empty() {
                                tracing::warn!(user_name = self.user_name, "User has no tasks.");

                                user.on_stop(&user_context).await;
                                return UserStatus::Finished;
                            }

                            loop {
                                if let Some(task) = tasks.get_prioritised_random() {
                                    let task_call_and_sleep = async {
                                        Test::sleep_between(self.between).await;
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
                    current_index += 1;
                }
            }
            supervisors
        })
    }
}
