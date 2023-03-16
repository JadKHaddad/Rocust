// Giving Tokio a break point to stop polling the future.
pub(crate) struct BreakPoint;

impl std::future::Future for BreakPoint {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::task::Poll::Pending
    }
}
