// use tokio::sync::mpsc::Sender;

pub struct SpawnCoordinator {
    pub users_per_sec: u64,
    pub user_spawn_controllers: Vec<UserSpawnController>,
}

pub struct UserSpawnController {
    pub user_name: String,
    pub count: u64,
    pub total_spawned: u64,
    //pub spawn_tx: Sender<u64>
}

impl SpawnCoordinator {
    // the spawners will wait for a signal to spawn a user
    pub async fn spawn(&mut self) {
        loop {
            // first of all we remove every controller that has already spawned all the users
            self.user_spawn_controllers.retain(|user_spawn_controller| {
                user_spawn_controller.total_spawned < user_spawn_controller.count
            });
            // break if there are no more controllers
            if self.user_spawn_controllers.is_empty() {
                break;
            }
            let mut seconds_to_wait = 1;

            // decide how many users should be spawned depending on the number of pending users
            let mut users_to_spawn_count =
                self.users_per_sec / self.user_spawn_controllers.len() as u64;
            if users_to_spawn_count == 0 {
                seconds_to_wait = self.user_spawn_controllers.len() as u64;
                users_to_spawn_count = self.users_per_sec;
            }
            let total = users_to_spawn_count * self.user_spawn_controllers.len() as u64;
            tracing::debug!(
                "{} users to spawn in {} seconds, total: {}",
                users_to_spawn_count,
                seconds_to_wait,
                total
            );
            // we still  dont want to spawn more users than the limit
            for user_spawn_controller in self.user_spawn_controllers.iter_mut() {
                user_spawn_controller.total_spawned += users_to_spawn_count;
                tracing::debug!(
                    "{users_to_spawn_count} sent to {} | total: {}, limit: {}",
                    user_spawn_controller.user_name,
                    user_spawn_controller.total_spawned,
                    user_spawn_controller.count
                );
                //user_spawn_controller.spawn_tx.send(users_to_spawn_count).await.unwrap();
            }
            self.user_spawn_controllers.retain(|user_spawn_controller| {
                user_spawn_controller.total_spawned < user_spawn_controller.count
            });
            // break if there are no more controllers
            if self.user_spawn_controllers.is_empty() {
                break;
            }
            // if the total is less than the limit, we need to spawn the remaining users
            // priority is given to the users that have the most pending users
            if total < self.users_per_sec {
                let mut remaining = self.users_per_sec - total;
                tracing::debug!("{} remaining users to spawn", remaining);
                for user_spawn_controller in self.user_spawn_controllers.iter_mut() {
                    if remaining == 0 {
                        break;
                    }
                    user_spawn_controller.total_spawned += 1;
                    remaining -= 1;
                    tracing::debug!(
                        "1 sent to {} | total: {}, limit: {}",
                        user_spawn_controller.user_name,
                        user_spawn_controller.total_spawned,
                        user_spawn_controller.count
                    );
                    //user_spawn_controller.spawn_tx.send(1).await.unwrap();
                }
            }
            tracing::debug!("-----------------------------");

            // now we sleep for 1 second
            tokio::time::sleep(std::time::Duration::from_secs(seconds_to_wait)).await;
        }
    }
}
