use std::{sync::Arc, time::Duration};

use tokio_util::sync::CancellationToken;

use crate::results::AllResults;

#[derive(Clone)]
pub struct TestController {
    token: Arc<CancellationToken>,
}

impl TestController {
    pub fn new(token: Arc<CancellationToken>) -> Self {
        TestController { token }
    }

    pub fn stop(&self) {
        tracing::info!("Stopping test");
        self.token.cancel();
    }

    pub(crate) async fn cancelled(&self) {
        self.token.cancelled().await
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
