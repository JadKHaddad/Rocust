use std::collections::HashMap;
use tokio::sync::mpsc::{error::SendError, UnboundedSender};

#[derive(Debug, Default, Clone)]
pub struct Results {
    pub total_requests: u32,
    pub total_failed_requests: u32,
    pub total_errors: u32,
    pub total_response_time: f64,
    pub average_response_time: f64,
    pub min_response_time: f64,
    pub median_response_time: f64,
    pub max_response_time: f64,
    pub requests_per_second: f64,
    pub failed_requests_per_second: f64,
}

impl Results {
    pub fn add_success(&mut self, response_time: f64) {
        self.total_response_time += response_time;
        self.total_requests += 1;
        self.average_response_time = self.total_response_time / self.total_requests as f64;
        if self.min_response_time == 0.0 || response_time < self.min_response_time {
            self.min_response_time = response_time;
        }
        if response_time > self.max_response_time {
            self.max_response_time = response_time;
        }
    }

    pub fn add_failure(&mut self) {
        self.total_requests += 1;
        self.total_failed_requests += 1;
    }

    pub fn add_error(&mut self) {
        self.total_errors += 1;
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct EndpointTypeName(String, String);

#[derive(Debug, Default, Clone)]
pub struct AllResults {
    pub aggrigated_results: Results,
    pub endpoint_results: HashMap<EndpointTypeName, Results>,
}

impl AllResults {
    pub fn add_success(&mut self, endpoint_type_name: EndpointTypeName, response_time: f64) {
        self.aggrigated_results.add_success(response_time);
        if let Some(endpoint_results) = self.endpoint_results.get_mut(&endpoint_type_name) {
            endpoint_results.add_success(response_time);
        } else {
            let mut endpoint_results = Results::default();
            endpoint_results.add_success(response_time);
            self.endpoint_results
                .insert(endpoint_type_name, endpoint_results);
        }
    }

    pub fn add_failure(&mut self, endpoint_type_name: EndpointTypeName) {
        self.aggrigated_results.add_failure();
        if let Some(endpoint_results) = self.endpoint_results.get_mut(&endpoint_type_name) {
            endpoint_results.add_failure();
        } else {
            let mut endpoint_results = Results::default();
            endpoint_results.add_failure();
            self.endpoint_results
                .insert(endpoint_type_name, endpoint_results);
        }
    }

    pub fn add_error(&mut self, endpoint_type_name: EndpointTypeName, _error: String) {
        self.aggrigated_results.add_error();
        if let Some(endpoint_results) = self.endpoint_results.get_mut(&endpoint_type_name) {
            endpoint_results.add_error();
        } else {
            let mut endpoint_results = Results::default();
            endpoint_results.add_error();
            self.endpoint_results
                .insert(endpoint_type_name, endpoint_results);
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ResultsSender {
    pub sender: Option<UnboundedSender<ResultMessage>>,
}

impl ResultsSender {
    pub fn new(sender: Option<UnboundedSender<ResultMessage>>) -> Self {
        Self { sender }
    }

    pub fn set_sender(&mut self, sender: UnboundedSender<ResultMessage>) {
        self.sender = Some(sender);
    }

    pub fn add_success(
        &self,
        r#type: String,
        name: String,
        response_time: f64,
    ) -> Result<(), SendError<ResultMessage>> {
        if let Some(sender) = &self.sender {
            return sender.send(ResultMessage::Success(SuccessResultMessage {
                endpoint_type_name: EndpointTypeName(r#type, name),
                response_time,
            }));
        }
        unreachable!();
    }

    pub async fn add_failure(
        &self,
        r#type: String,
        name: String,
    ) -> Result<(), SendError<ResultMessage>> {
        if let Some(sender) = &self.sender {
            return sender.send(ResultMessage::Failure(FailureResultMessage {
                endpoint_type_name: EndpointTypeName(r#type, name),
            }));
        }
        unreachable!();
    }

    pub async fn add_error(
        &self,
        r#type: String,
        name: String,
        error: String,
    ) -> Result<(), SendError<ResultMessage>> {
        if let Some(sender) = &self.sender {
            return sender.send(ResultMessage::Error(ErrorResultMessage {
                endpoint_type_name: EndpointTypeName(r#type, name),
                error,
            }));
        }
        unreachable!();
    }
}

pub enum ResultMessage {
    Success(SuccessResultMessage),
    Failure(FailureResultMessage),
    Error(ErrorResultMessage),
}

pub struct SuccessResultMessage {
    pub endpoint_type_name: EndpointTypeName,
    pub response_time: f64,
}

pub struct FailureResultMessage {
    pub endpoint_type_name: EndpointTypeName,
}

pub struct ErrorResultMessage {
    pub endpoint_type_name: EndpointTypeName,
    pub error: String,
}
