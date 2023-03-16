use std::sync::Arc;
use tokio_util::sync::CancellationToken;

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
