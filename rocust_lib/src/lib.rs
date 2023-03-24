pub mod events;
pub(crate) mod fs;
pub mod futures;
pub(crate) mod messages;
pub(crate) mod prometheus_exporter;
pub mod results;
pub(crate) mod server;
pub mod tasks;
pub mod test;
pub mod traits;
pub(crate) mod utils;

pub use test::{config::TestConfig, user::context::Context, Test};
pub use traits::{Shared, User};

#[macro_export]
macro_rules! run {
    ($test:ident, $user_type:ty $(,$user_types:ty)*) => {
        async {
            let (results_tx, results_rx) = $test.before_spawn_users().await;

            // create the shared data for the Data struct for each user
            let test_controller = std::sync::Arc::new($test.create_test_controller());

            // get the shared data from the first user type
            let shared = <$user_type as rocust::rocust_lib::traits::User>::Shared::new().await;

            // decide the weight of each user type and spawn accordingly
            let mut weights = std::collections::HashMap::new();
            weights.insert(stringify!(<$user_type>), <$user_type as rocust::rocust_lib::traits::HasTask>::get_weight());
            $(
                weights.insert(stringify!(<$user_types>), <$user_types as rocust::rocust_lib::traits::HasTask>::get_weight());
            )*
            let total_given_users_count = weights.len();
            let full_weight = weights.iter().map(|(_, weight)| weight).sum::<u64>();
            let counts = weights.iter().map(|(name, weight)| (name, $test.get_config().user_count * weight/full_weight)).collect::<std::collections::HashMap<_,_>>();

            // depending on the weights, some users may not be spawned
            let total_spawnable_user_count = counts.iter().map(|(_, count)| count).sum::<u64>();

            let mut user_spawn_controllers = Vec::new();
            let mut spawn_users_handles_vec = Vec::new();

            // how much to spawn and index interval as parameters
            let mut start_index = 0;
            let spawn_count = counts.get(&stringify!(<$user_type>)).expect("Unreachable Macro error!").clone();
            // create a spawnController
            // create an unbounded channel for the SpawnCoordinator and the Spawners
            let (tx, rx) = tokio::sync::mpsc::channel(100);
            user_spawn_controllers.push(rocust::rocust_lib::test::spawn_coordinator::UserSpawnController::new(
                <$user_type as rocust::rocust_lib::traits::HasTask>::get_name(),
                spawn_count,
                tx
            ));
            //create the spawner
            let spawner: rocust::rocust_lib::test::spawn_coordinator::Spawner::<$user_type, <$user_type as rocust::rocust_lib::traits::User>::Shared>
            = rocust::rocust_lib::test::spawn_coordinator::Spawner::new(
                spawn_count,
                $test.clone_token(),
                $test.get_config().clone(),
                test_controller.clone(),
                results_tx.clone(),
                start_index,
                shared.clone(),
                rx
            );
            spawn_users_handles_vec.push(
                spawner.run()
            );
            start_index += spawn_count;

            $(
                let spawn_count = counts.get(&stringify!(<$user_types>)).expect("Unreachable Macro error!").clone();
                let (tx, rx) = tokio::sync::mpsc::channel(100);
                user_spawn_controllers.push(rocust::rocust_lib::test::spawn_coordinator::UserSpawnController::new(
                    <$user_types as rocust::rocust_lib::traits::HasTask>::get_name(),
                    spawn_count,
                    tx
                ));
                let spawner: rocust::rocust_lib::test::spawn_coordinator::Spawner::<$user_types, <$user_types as rocust::rocust_lib::traits::User>::Shared>
                = rocust::rocust_lib::test::spawn_coordinator::Spawner::new(
                    spawn_count,
                    $test.clone_token(),
                    $test.get_config().clone(),
                    test_controller.clone(),
                    results_tx.clone(),
                    start_index,
                    shared.clone(),
                    rx
                );
                spawn_users_handles_vec.push(
                    spawner.run()
                );
                start_index += spawn_count;
            )*

            // now we can start the spawn coordinator
            let spawn_coordinator = rocust::rocust_lib::test::spawn_coordinator::SpawnCoordinator::new(
                $test.get_config().users_per_sec,
                user_spawn_controllers,
                $test.clone_token()
            );

            // drop because why not >:D
            drop(test_controller);

            // drop the events_handler to drop the sender
            drop(results_tx);
            $test.after_spawn_users(results_rx, spawn_coordinator, spawn_users_handles_vec, total_spawnable_user_count).await;
        }
    };
}
