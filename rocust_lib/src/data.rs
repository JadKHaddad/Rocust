use crate::{
    events::EventsHandler, results::AllResults, test::TestController, test_config::TestConfig,
    user::UserController,
};
use std::{sync::Arc, time::Duration};

pub struct Data {
    // each user will recieve a Data obj containing

    // arc because it is shared between all users
    test_controller: Arc<TestController>,

    // arc because it is shared between all users
    test_config: Arc<TestConfig>,

    // arc because it is shared between all users
    events_handler: Arc<EventsHandler>,

    // not shared between users
    user_controller: UserController,
    // why is AllResults not included here?
    // well, because it is behind an RwLock, wich is only accessed in 3 main tasks (test server, test main loop, test background loop)
    // we don't want the ~1000 users to accuire a lock on it with on every single task
    // the user could track his own results in his internal state or use the shared object accross all users
}

impl Data {
    pub fn new(
        test_controller: Arc<TestController>,
        test_config: Arc<TestConfig>,
        events_handler: Arc<EventsHandler>,
        user_controller: UserController,
    ) -> Self {
        Self {
            test_controller,
            test_config,
            events_handler,
            user_controller,
        }
    }

    pub fn get_test_controller(&self) -> &Arc<TestController> {
        &self.test_controller
    }

    pub fn get_test_config(&self) -> &Arc<TestConfig> {
        &self.test_config
    }

    pub fn get_events_handler(&self) -> &Arc<EventsHandler> {
        &self.events_handler
    }

    pub fn get_user_controller(&self) -> &UserController {
        &self.user_controller
    }
}

pub struct StopConditionData<'a> {
    all_results: &'a AllResults,
    elapsed_time: &'a Duration,
}

impl<'a> StopConditionData<'a> {
    pub fn new(all_results: &'a AllResults, elapsed_time: &'a Duration) -> Self {
        Self {
            all_results,
            elapsed_time,
        }
    }

    pub fn get_all_results(&self) -> &AllResults {
        self.all_results
    }

    pub fn get_elapsed_time(&self) -> &Duration {
        self.elapsed_time
    }
}
