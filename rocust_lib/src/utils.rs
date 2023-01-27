use crate::traits::Prioritised;
use rand::{distributions::WeightedIndex, prelude::Distribution};

pub fn choose_random_object<T>(objects: &Vec<T>) -> Option<&T>
where
    T: Prioritised,
{
    let weights: Vec<i32> = objects.iter().map(|o| o.get_priority()).collect();
    let distrib = WeightedIndex::new(weights).unwrap();
    let mut rng = rand::thread_rng();
    let idx = distrib.sample(&mut rng);
    objects.get(idx)
}
