// Giving Tokio a break point to stop polling the future.
pub(crate) struct FakeFuture;

impl std::future::Future for FakeFuture {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        cx.waker().wake_by_ref();
        std::task::Poll::Pending
    }
}
