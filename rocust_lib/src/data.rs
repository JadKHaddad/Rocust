use crate::{
    events::EventsHandler,
    results::AllResults,
    test::{TestConfig, TestController},
};
use std::time::Duration;

pub struct Data {
    pub test_controller: TestController,
    pub test_config: TestConfig,
    pub events_handler: EventsHandler,
    // why is AllResults not included here?
    // well, because it is behind an RwLock, wich is only accessed in two main tasks
    // we don't want the ~1000 users to accuire a lock on it with on every single task
    // the user could track his own results in his internal state or use the shared object accross all users
}

impl Data {
    pub fn new(
        test_controller: TestController,
        test_config: TestConfig,
        events_handler: EventsHandler,
    ) -> Self {
        Self {
            test_controller,
            test_config,
            events_handler,
        }
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
