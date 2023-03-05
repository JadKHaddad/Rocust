use crate::{test::config::TestConfig, test::user::context::Context};
use async_trait::async_trait;
use rand::{distributions::WeightedIndex, prelude::Distribution};

pub trait HasTask: 'static {
    fn get_async_tasks() -> Vec<crate::tasks::AsyncTask<Self>>
    where
        Self: Sized,
    {
        vec![]
    }

    fn get_name() -> String {
        String::new()
    }

    fn get_between() -> (u64, u64) {
        (0, 0)
    }

    fn get_weight() -> u64 {
        1
    }
}

#[async_trait]
pub trait User: Send + Sized + 'static {
    type Shared: Shared;

    async fn new(_test_config: &TestConfig, _data: &Context, _shared: Self::Shared) -> Self;
    async fn on_start(&mut self, _data: &Context) {}
    async fn on_stop(&mut self, _data: &Context) {}
}

#[async_trait]
pub trait Shared: Clone + Send + 'static {
    async fn new() -> Self;
}

pub trait Prioritised {
    fn get_priority(&self) -> u64;
}

pub trait PrioritisedRandom<T>
where
    T: Prioritised,
{
    fn get_prioritised_random(&self) -> Option<&T>;
}

impl<T> PrioritisedRandom<T> for Vec<T>
where
    T: Prioritised,
{
    fn get_prioritised_random(&self) -> Option<&T> {
        let weights: Vec<u64> = self.iter().map(|o| o.get_priority()).collect();
        if let Ok(distrib) = WeightedIndex::new(weights) {
            let mut rng = rand::thread_rng();
            let idx = distrib.sample(&mut rng);
            return self.get(idx);
        }
        None
    }
}

#[async_trait]
impl Shared for () {
    async fn new() -> Self {}
}
