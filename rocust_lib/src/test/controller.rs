use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct TestController {
    token: CancellationToken,
}

impl TestController {
    pub fn new(token: CancellationToken) -> Self {
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
