use async_trait::async_trait;
use std::sync::Arc;
pub trait HasTask {
    fn get_async_tasks() -> Vec<crate::tasks::AsyncTask<Self>>
    where
        Self: Sized,
    {
        vec![]
    }

    fn get_name() -> String {
        String::from("unnamed")
    }

    fn get_between() -> (u64, u64) {
        (0, 0)
    }

    fn get_weight() -> u64 {
        1
    }
}
#[async_trait]
pub trait User: Send + Sized {
    type Shared: Shared;

    async fn new(_id: u64, _data: &Arc<Data>, _shared: Self::Shared) -> Self;
    async fn on_start(&mut self, _data: &Arc<Data>) {}
    async fn on_stop(&mut self, _data: &Arc<Data>) {}
}

#[async_trait]
pub trait Shared: Clone + Send {
    async fn new() -> Self;
}

pub trait Prioritised {
    fn get_priority(&self) -> u64;
}

pub trait PrioritisedRandom<T>
where
    T: Prioritised,
{
    fn get_proioritised_random(&self) -> Option<&T>;
}

use rand::{distributions::WeightedIndex, prelude::Distribution};

use crate::data::Data;

impl<T> PrioritisedRandom<T> for Vec<T>
where
    T: Prioritised,
{
    fn get_proioritised_random(&self) -> Option<&T> {
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
