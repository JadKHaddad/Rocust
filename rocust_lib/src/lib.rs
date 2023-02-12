pub mod events;
pub mod results;
pub mod tasks;
pub mod test;
pub mod traits;

#[macro_export]
macro_rules! run {
    ($test:ident, $($user_type:ty),+) => {
        let (results_tx, results_rx) = $test.before_spawn_users().await;

        //TODO: decide the weight of each user type and spawn accordingly
        let mut spawn_users_handles_vec = Vec::new();
        $(
            //TODO: how much to spawn und index interval as parameters
            let spawn_users_handles = $test.spawn_users::<$user_type>(results_tx.clone());
            spawn_users_handles_vec.push(spawn_users_handles);
        )*

        $test.after_spawn_users(results_tx, results_rx, spawn_users_handles_vec)
            .await;
    };
}
