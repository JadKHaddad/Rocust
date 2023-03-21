use super::controller::TestController;
use crate::{
    events::EventsHandler,
    messages::MainMessage,
    tasks::{AsyncTask, EventsTaskInfo},
    test::user::{EventsUserInfo, UserController, UserStatus},
    traits::{HasTask, PrioritisedRandom},
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
    count: u64,
    total_spawned: u64,
    spawn_tx: UnboundedSender<u64>,
}

impl UserSpawnController {
    pub fn new(user_name: &'static str, count: u64, spawn_tx: UnboundedSender<u64>) -> Self {
        Self {
            user_name,
            count,
            total_spawned: 0,
            spawn_tx,
        }
    }

    fn send_spawn_count(&mut self, count: u64) {
        let _ = self.spawn_tx.send(count);
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

    fn inner_spawn(&mut self, global_users_to_spawn_per_user: f64) {
        // now we check if all the users can be spawned in the gevin time and if not we save the remainning users to spawn
        let mut remaining_users_to_spawn = 0;

        self.user_spawn_controllers
            .retain_mut(|user_spawn_controller| {
                let mut users_to_spawn_count_per_user = global_users_to_spawn_per_user;
                let mut retain = true;
                if user_spawn_controller.total_spawned + global_users_to_spawn_per_user as u64
                    >= user_spawn_controller.count
                {
                    users_to_spawn_count_per_user =
                        (user_spawn_controller.count - user_spawn_controller.total_spawned) as f64;
                    remaining_users_to_spawn += user_spawn_controller.total_spawned
                        + global_users_to_spawn_per_user as u64
                        - user_spawn_controller.count;
                    // so now this guy has spawned all the users, lets remove him from the list
                    retain = false;
                }
                user_spawn_controller.send_spawn_count(users_to_spawn_count_per_user as u64);
                user_spawn_controller.total_spawned += users_to_spawn_count_per_user as u64;

                tracing::debug!(
                    count = users_to_spawn_count_per_user as u64,
                    user_name = user_spawn_controller.user_name,
                    total_users_spawned = user_spawn_controller.total_spawned,
                    total_spawnable_users = user_spawn_controller.count,
                    "Spawning users"
                );

                retain
            });

        // now we could have some remaining users to spawn, so lets spawn them
        if remaining_users_to_spawn > 0 {
            let global_users_to_spawn_count_per_user =
                remaining_users_to_spawn as f64 / self.user_spawn_controllers.len() as f64;
            self.inner_spawn(global_users_to_spawn_count_per_user);
        }
    }

    async fn spawn(&mut self) {
        loop {
            if self.user_spawn_controllers.is_empty() {
                break;
            }

            let mut wait_time_in_millis = 1000;

            // decide how many users per user type should be spawned depending on the number of pending users
            let mut users_to_spawn =
                self.users_per_sec as f64 / self.user_spawn_controllers.len() as f64;

            // FIXME adjust the wait time if we have a number of users to spawn that is less than not a whole number
            if users_to_spawn.fract() > 0.0 {
                wait_time_in_millis = (1000.0 / users_to_spawn.fract()) as u64;
                users_to_spawn = users_to_spawn.floor();
            }

            tracing::debug!(
                %users_to_spawn,
                %wait_time_in_millis,
                "Spawning users"
            );

            self.inner_spawn(users_to_spawn);

            tokio::time::sleep(std::time::Duration::from_millis(wait_time_in_millis)).await;
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
