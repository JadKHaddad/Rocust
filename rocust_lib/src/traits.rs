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

pub trait User: Send + Sized {
    type Shared: Shared;

    fn new(_id: u64, _handler: &EventsHandler, _shared: Self::Shared) -> Self; // TODO: pass test config and a test controller to be able to stop the test based on some user defined conditions
    fn on_start(&mut self, _handler: &EventsHandler) {}
    fn on_stop(&mut self, _handler: &EventsHandler) {}
}

pub trait Shared: Clone + Send {
    fn new() -> Self;
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

use crate::events::EventsHandler;
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

impl Shared for () {
    fn new() -> Self {}
}
