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
    fn inner_spawn(&mut self, global_users_to_spawn_count_per_user: f64) {
        // now we check if all the users can be spawned in the gevin time and if not we save the remainning users to spawn
        let mut remaining_users_to_spawn = 0;

        self.user_spawn_controllers
            .retain_mut(|user_spawn_controller| {
                let mut users_to_spawn_count_per_user = global_users_to_spawn_count_per_user;
                let mut retain = true;
                if user_spawn_controller.total_spawned + global_users_to_spawn_count_per_user as u64
                    >= user_spawn_controller.count
                {
                    users_to_spawn_count_per_user =
                        (user_spawn_controller.count - user_spawn_controller.total_spawned) as f64;
                    remaining_users_to_spawn += user_spawn_controller.total_spawned
                        + global_users_to_spawn_count_per_user as u64
                        - user_spawn_controller.count;
                    // so now this guy has spawned all the users, lets remove him from the list
                    retain = false;
                }
                user_spawn_controller.total_spawned += users_to_spawn_count_per_user as u64;
                tracing::debug!(
                    "Spawning {} users of type {} | total: {} | limit: {}",
                    users_to_spawn_count_per_user,
                    user_spawn_controller.user_name,
                    user_spawn_controller.total_spawned,
                    user_spawn_controller.count
                );
                retain
            });

        // now we could have some remaining users to spawn, so lets spawn them
        if remaining_users_to_spawn > 0 {
            tracing::debug!("{} remaining users to spawn", remaining_users_to_spawn);
            let global_users_to_spawn_count_per_user =
                remaining_users_to_spawn as f64 / self.user_spawn_controllers.len() as f64;
            self.inner_spawn(global_users_to_spawn_count_per_user.floor());
        }
    }

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

            let mut milli_seconds_to_wait = 1000;

            // decide how many users per user type should be spawned depending on the number of pending users
            let mut users_to_spawn_count =
                self.users_per_sec as f64 / self.user_spawn_controllers.len() as f64;

            // adjust the wait time if we have to spawn a number of users that is not an integer
            if users_to_spawn_count.fract() != 0.0 {
                milli_seconds_to_wait = (1000.0 / users_to_spawn_count.fract()) as u64;
                users_to_spawn_count = users_to_spawn_count.ceil();
            }

            tracing::debug!(
                "Spawning {} users per {} milli second",
                users_to_spawn_count,
                milli_seconds_to_wait
            );

            self.inner_spawn(users_to_spawn_count);
            tracing::debug!("---------------------------------------------------");

            // now we sleep
            tokio::time::sleep(std::time::Duration::from_millis(milli_seconds_to_wait)).await;
        }
    }
}
