use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Clone, Default)]
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
    pub fn add_succ(&mut self, _dummy: i32) {}

    pub fn add_fail(&mut self, _dummy: i32) {}
}

pub struct ResultReceiver {
    pub results: Results,
    pub receiver: Receiver<ResultMessage>,
}

impl ResultReceiver {
    pub fn new(results: Results, receiver: Receiver<ResultMessage>) -> Self {
        Self { results, receiver }
    }
}

#[derive(Default)]
pub struct ResultSender {
    pub sender: Option<Sender<ResultMessage>>,
}

impl ResultSender {
    pub fn new(sender: Option<Sender<ResultMessage>>) -> Self {
        Self { sender }
    }

    pub fn set_sender(&mut self, sender: Sender<ResultMessage>) {
        self.sender = Some(sender);
    }

    pub async fn send_success(
        &self,
        r#type: String,
        name: String,
        response_time: f64,
    ) -> Result<(), SendError<ResultMessage>> {
        if let Some(sender) = &self.sender {
            return sender
                .send(ResultMessage::Success(SuccessResultMessage {
                    r#type,
                    name,
                    response_time,
                }))
                .await;
        }
        unreachable!();
    }

    pub async fn send_fail(
        &self,
        r#type: String,
        name: String,
    ) -> Result<(), SendError<ResultMessage>> {
        if let Some(sender) = &self.sender {
            return sender
                .send(ResultMessage::Fail(FailResultMessage { r#type, name }))
                .await;
        }
        unreachable!();
    }

    pub async fn send_error(
        &self,
        r#type: String,
        name: String,
        error: String,
    ) -> Result<(), SendError<ResultMessage>> {
        if let Some(sender) = &self.sender {
            return sender
                .send(ResultMessage::Error(ErrorResultMessage {
                    r#type,
                    name,
                    error,
                }))
                .await;
        }
        unreachable!();
    }
}

pub enum ResultMessage {
    Success(SuccessResultMessage),
    Fail(FailResultMessage),
    Error(ErrorResultMessage),
}

pub struct SuccessResultMessage {
    pub r#type: String,
    pub name: String,
    pub response_time: f64,
}

pub struct FailResultMessage {
    pub r#type: String,
    pub name: String,
}

pub struct ErrorResultMessage {
    pub r#type: String,
    pub name: String,
    pub error: String,
}
