use crate::{events::EventsHandler, test::controller::TestController, test::user::UserController};
use std::sync::Arc;

pub struct Context {
    // each user will recieve a Data obj containing

    // arc because it is shared between all users
    test_controller: Arc<TestController>,

    // not shared between users
    events_handler: EventsHandler,

    // not shared between users
    user_controller: UserController,
    // why is AllResults not included here?
    // well, because it is behind an RwLock, wich is only accessed in 3 main tasks (test server, test main loop, test background loop)
    // we don't want the ~1000 users to accuire a lock on it with on every single task
    // the user could track his own results in his internal state or use the shared object accross all users
}

impl Context {
    pub fn new(
        test_controller: Arc<TestController>,
        events_handler: EventsHandler,
        user_controller: UserController,
    ) -> Self {
        Self {
            test_controller,
            events_handler,
            user_controller,
        }
    }

    pub fn stop(&self) {
        self.user_controller.stop();
    }

    pub fn stop_test(&self) {
        self.test_controller.stop();
    }

    pub fn add_success(&self, r#type: String, name: String, response_time: f64) {
        self.events_handler.add_success(r#type, name, response_time);
    }

    pub fn add_failure(&self, r#type: String, name: String) {
        self.events_handler.add_failure(r#type, name);
    }

    pub fn add_error(&self, r#type: String, name: String, error: String) {
        self.events_handler.add_error(r#type, name, error);
    }

    pub(crate) fn get_events_handler(&self) -> &EventsHandler {
        &self.events_handler
    }

    pub fn get_id(&self) -> u64 {
        self.events_handler.get_user_id()
    }
}
