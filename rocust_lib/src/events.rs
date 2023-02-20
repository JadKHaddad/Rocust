use crate::{
    messages::{
        ErrorResultMessage, FailureResultMessage, MainMessage, ResultMessage, SuccessResultMessage,
        UserSpawnedMessage,
    },
    results::EndpointTypeName,
};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone)]
pub struct EventsHandler {
    sender: UnboundedSender<MainMessage>,
}

impl EventsHandler {
    pub fn new(sender: UnboundedSender<MainMessage>) -> Self {
        Self { sender }
    }

    pub fn add_success(&self, r#type: String, name: String, response_time: f64) {
        let _ = self
            .sender
            .send(MainMessage::ResultMessage(ResultMessage::Success(
                SuccessResultMessage {
                    endpoint_type_name: EndpointTypeName(r#type, name),
                    response_time,
                },
            )));
    }

    pub fn add_failure(&self, r#type: String, name: String) {
        let _ = self
            .sender
            .send(MainMessage::ResultMessage(ResultMessage::Failure(
                FailureResultMessage {
                    endpoint_type_name: EndpointTypeName(r#type, name),
                },
            )));
    }

    pub fn add_error(&self, r#type: String, name: String, error: String) {
        let _ = self
            .sender
            .send(MainMessage::ResultMessage(ResultMessage::Error(
                ErrorResultMessage {
                    endpoint_type_name: EndpointTypeName(r#type, name),
                    error,
                },
            )));
    }

    pub(crate) fn add_user_spawned(&self, id: u64, name: String) {
        let _ = self
            .sender
            .send(MainMessage::UserSpawned(UserSpawnedMessage { id, name }));
    }
}
