use crate::{
    messages::{
        ErrorResultMessage, FailureResultMessage, MainMessage, ResultMessage, SuccessResultMessage,
        UserSpawnedMessage,
    },
    results::EndpointTypeName,
    user::EventsUserInfo,
};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone)]
pub struct EventsHandler {
    user_info: EventsUserInfo,
    sender: UnboundedSender<MainMessage>,
}

impl EventsHandler {
    pub fn new(user_info: EventsUserInfo, sender: UnboundedSender<MainMessage>) -> Self {
        Self { user_info, sender }
    }

    pub fn add_success(&self, r#type: String, name: String, response_time: f64) {
        let _ = self
            .sender
            .send(MainMessage::ResultMessage(ResultMessage::Success(
                SuccessResultMessage {
                    user_id: self.user_info.id,
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
                    user_id: self.user_info.id,
                    endpoint_type_name: EndpointTypeName(r#type, name),
                },
            )));
    }

    pub fn add_error(&self, r#type: String, name: String, error: String) {
        let _ = self
            .sender
            .send(MainMessage::ResultMessage(ResultMessage::Error(
                ErrorResultMessage {
                    user_id: self.user_info.id,
                    endpoint_type_name: EndpointTypeName(r#type, name),
                    error,
                },
            )));
    }

    pub(crate) fn add_user_spawned(&self) {
        let _ = self
            .sender
            .send(MainMessage::UserSpawned(UserSpawnedMessage {
                user_info: self.user_info.clone(),
            }));
    }

    pub fn get_user_id(&self) -> u64 {
        self.user_info.id
    }
}
