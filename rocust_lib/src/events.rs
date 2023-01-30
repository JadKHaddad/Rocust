use crate::traits::HasTask;

pub fn add_success(user: &impl HasTask, r#type: String, name: String, response_time: f64) {
    let results_sender = user.get_results_sender();
    let _ = results_sender.add_success(r#type, name, response_time);
}

pub fn add_failure(user: &impl HasTask, r#type: String, name: String) {
    let results_sender = user.get_results_sender();
    let _ = results_sender.add_failure(r#type, name);
}

pub fn add_error(user: &impl HasTask, r#type: String, name: String, error: String) {
    let results_sender = user.get_results_sender();
    let _ = results_sender.add_error(r#type, name, error);
}
