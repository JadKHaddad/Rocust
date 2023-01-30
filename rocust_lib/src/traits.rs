use crate::results::ResultMessage;
use tokio::sync::mpsc::UnboundedSender;

pub trait HasTask {
    fn get_async_tasks() -> Vec<crate::tasks::AsyncTask<Self>>
    where
        Self: Sized,
    {
        vec![]
    }

    fn get_results_sender(&self) -> &ResultsSender;

    fn set_sender(&mut self, sender: UnboundedSender<ResultMessage>);
}

pub trait User {
    fn on_create(&mut self, _id: u16) {}
    fn on_start(&mut self) {}
    fn on_stop(&mut self) {}
}

pub trait Prioritised {
    fn get_priority(&self) -> i32;
}

pub trait PrioritisedRandom<T>
where
    T: Prioritised,
{
    fn get_proioritised_random(&self) -> Option<&T>;
}

use rand::{distributions::WeightedIndex, prelude::Distribution};

use crate::results::ResultsSender;
impl<T> PrioritisedRandom<T> for Vec<T>
where
    T: Prioritised,
{
    fn get_proioritised_random(&self) -> Option<&T> {
        let weights: Vec<i32> = self.iter().map(|o| o.get_priority()).collect();
        if let Ok(distrib) = WeightedIndex::new(weights) {
            let mut rng = rand::thread_rng();
            let idx = distrib.sample(&mut rng);
            return self.get(idx);
        }
        None
    }
}
