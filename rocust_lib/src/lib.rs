pub mod results;
pub mod tasks;
pub mod test;
pub mod traits;

#[macro_export]
macro_rules! run {
    ($test:ident, $user_type:ty $(,$user_types:ty),*) => {
        let (results_tx, results_rx) = $test.before_spawn_users().await;
        let events_handler = EventsHandler::new(results_tx);

        let mut spawn_users_handles_vec = Vec::new();

        // get the shared data from the first user type
        let shared = <$user_type as User>::Shared::new();

        //TODO: decide the weight of each user type and spawn accordingly
        //TODO: how much to spawn and index interval as parameters

        let spawn_users_handles = $test.spawn_users::<$user_type, <$user_type as User>::Shared>(events_handler.clone(), shared.clone());
        spawn_users_handles_vec.push(spawn_users_handles);

        $(
            let spawn_users_handles = $test.spawn_users::<$user_types, <$user_types as User>::Shared>(events_handler.clone(), shared.clone());
            spawn_users_handles_vec.push(spawn_users_handles);
        )*

        $test.after_spawn_users(events_handler, results_rx, spawn_users_handles_vec)
            .await;
    };
}
