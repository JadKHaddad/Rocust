pub mod data;
pub mod events;
pub(crate) mod messages;
pub mod results;
pub(crate) mod server;
pub mod tasks;
pub mod test;
pub mod test_config;
pub mod traits;
pub(crate) mod user;
pub(crate) mod utils;
pub(crate) mod writer;

#[macro_export]
macro_rules! run {
    ($test:ident, $user_type:ty $(,$user_types:ty)*) => {
        {
            let (results_tx, results_rx) = $test.before_spawn_users().await;
            let events_handler = EventsHandler::new(results_tx);

            let data = rocust::rocust_lib::data::Data::new($test.create_test_controller(), $test.get_config().clone(), events_handler.clone());
            let data_arc = std::sync::Arc::new(data);

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

            let mut spawn_users_handles_vec = Vec::new();

            // how much to spawn and index interval as parameters
            let mut start_index = 0;
            let spawn_count = counts.get(&stringify!(<$user_type>)).expect("Unreachable Macro error!").clone();
            let spawn_users_handles = $test.spawn_users::<$user_type, <$user_type as rocust::rocust_lib::traits::User>::Shared>(spawn_count,start_index, data_arc.clone(), shared.clone());
            spawn_users_handles_vec.push(spawn_users_handles);
            start_index += spawn_count;

            $(
                let spawn_count = counts.get(&stringify!(<$user_types>)).expect("Unreachable Macro error!").clone();
                let spawn_users_handles = $test.spawn_users::<$user_types, <$user_types as rocust::rocust_lib::traits::User>::Shared>(spawn_count,start_index, data_arc.clone(), shared.clone());
                spawn_users_handles_vec.push(spawn_users_handles);
                start_index += spawn_count;
            )*

            drop(shared);
            $test.after_spawn_users(events_handler, results_rx, spawn_users_handles_vec)
        }
    };
}
