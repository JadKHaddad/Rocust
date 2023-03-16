use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

// Giving Tokio a break point to stop polling the future.
pub(crate) struct BreakPoint;

impl Future for BreakPoint {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

use pin_project::pin_project;
use tokio::time::Instant;

#[pin_project]
pub struct Timed<Fut>
where
    Fut: Future,
{
    #[pin]
    inner: Fut,
    start: Option<Instant>,
}

impl<Fut> Future for Timed<Fut>
where
    Fut: Future,
{
    type Output = (Fut::Output, Duration);

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        let start = this.start.get_or_insert_with(Instant::now);

        match this.inner.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(v) => {
                let elapsed = start.elapsed();
                Poll::Ready((v, elapsed))
            }
        }
    }
}

pub trait TimedExt: Sized + Future {
    fn timed(self) -> Timed<Self> {
        Timed {
            inner: self,
            start: None,
        }
    }
}

// All futures can use the `.timed` method defined above
impl<F: Future> TimedExt for F {}
