use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

// Giving Tokio a breakpoint to stop polling the future.
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
            Poll::Ready(value) => {
                let elapsed = start.elapsed();
                Poll::Ready((value, elapsed))
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

#[pin_project]
pub struct Counted<Fut>
where
    Fut: Future,
{
    #[pin]
    inner: Fut,
    polls: u32,
}

impl<Fut> Future for Counted<Fut>
where
    Fut: Future,
{
    type Output = (Fut::Output, u32);

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        *this.polls += 1;

        match this.inner.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(value) => Poll::Ready((value, *this.polls)),
        }
    }
}

pub trait CountedExt: Sized + Future {
    fn counted(self) -> Counted<Self> {
        Counted {
            inner: self,
            polls: 0,
        }
    }
}

// All futures can use the `.counted` method defined above
impl<F: Future> CountedExt for F {}

use tokio::time::Sleep;

#[pin_project]
pub struct Delayed<Fut>
where
    Fut: Future,
{
    #[pin]
    inner: Fut,
    #[pin]
    delay: Sleep,
    delay_ready: bool,
}

impl<Fut> Future for Delayed<Fut>
where
    Fut: Future,
{
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();

        if !*this.delay_ready {
            if let Poll::Ready(_) = this.delay.poll(cx) {
                *this.delay_ready = true;
                cx.waker().wake_by_ref();
            }
            return Poll::Pending;
        }

        this.inner.poll(cx)
    }
}

pub trait DelayedExt: Sized + Future {
    fn delayed(self, duration: Duration) -> Delayed<Self> {
        Delayed {
            inner: self,
            delay: tokio::time::sleep(duration),
            delay_ready: false,
        }
    }
}

// All futures can use the `.delayed` method defined above
impl<F: Future> DelayedExt for F {}

pub trait RocustFutures: TimedExt + CountedExt + DelayedExt + Sized + Future {
    fn timed(self) -> Timed<Self> {
        TimedExt::timed(self)
    }

    fn counted(self) -> Counted<Self> {
        CountedExt::counted(self)
    }

    fn delayed(self, duration: Duration) -> Delayed<Self> {
        DelayedExt::delayed(self, duration)
    }
}

// All futures can use the `.timed, .counted and .delayed` methods defined above in one trait
impl<F: Future> RocustFutures for F {}
