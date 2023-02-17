use tokio::sync::mpsc::UnboundedSender;
use crate::{
    messages::{
        ErrorResultMessage, FailureResultMessage, MainMessage, ResultMessage, SuccessResultMessage,
    },
    results::EndpointTypeName,
};

#[derive(Debug, Clone)]
pub struct EventsHandler {
    pub(crate) sender: UnboundedSender<MainMessage>,
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
}
