pub trait HasTask {
    fn add_succ(&mut self, dummy: i32);
    fn add_fail(&mut self, dummy: i32);
    fn get_async_tasks() -> Vec<crate::tasks::AsyncTask<Self>>
    where
        Self: Sized,
    {
        vec![]
    }
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
