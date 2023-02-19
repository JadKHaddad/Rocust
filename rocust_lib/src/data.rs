use crate::{
    events::EventsHandler,
    test::{TestConfig, TestController},
};

pub struct Data {
    pub test_controller: TestController,
    pub test_config: TestConfig,
    pub events_handler: EventsHandler,
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
