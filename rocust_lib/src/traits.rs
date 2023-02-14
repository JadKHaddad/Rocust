pub trait HasTask {
    fn get_async_tasks() -> Vec<crate::tasks::AsyncTask<Self>>
    where
        Self: Sized,
    {
        vec![]
    }

    fn get_between() -> (u64, u64) {
        (0, 0)
    }

    fn get_weight() -> u64 {
        1
    }
}

pub trait User {
    fn new(_id: u16, _handler: &EventsHandler) -> Self
    where
        Self: Sized;
    fn on_start(&mut self, _handler: &EventsHandler) {}
    fn on_stop(&mut self, _handler: &EventsHandler) {}
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

use crate::results::EventsHandler;
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
