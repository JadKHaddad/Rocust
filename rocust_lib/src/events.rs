use crate::{
    messages::{
        ErrorResultMessage, FailureResultMessage, MainMessage, ResultMessage, SuccessResultMessage,
        TaskExecutedMessage, UserSelfStoppedMessage, UserSpawnedMessage,
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

    fn send(&self, message: MainMessage) {
        let _ = self.sender.send(message);
    }

    pub fn add_success(&self, r#type: String, name: String, response_time: f64) {
        self.send(MainMessage::ResultMessage(ResultMessage::Success(
            SuccessResultMessage {
                user_id: self.user_info.id,
                endpoint_type_name: EndpointTypeName(r#type, name),
                response_time,
            },
        )));
    }

    pub fn add_failure(&self, r#type: String, name: String) {
        self.send(MainMessage::ResultMessage(ResultMessage::Failure(
            FailureResultMessage {
                user_id: self.user_info.id,
                endpoint_type_name: EndpointTypeName(r#type, name),
            },
        )));
    }

    pub fn add_error(&self, r#type: String, name: String, error: String) {
        self.send(MainMessage::ResultMessage(ResultMessage::Error(
            ErrorResultMessage {
                user_id: self.user_info.id,
                endpoint_type_name: EndpointTypeName(r#type, name),
                error,
            },
        )));
    }

    pub(crate) fn add_user_spawned(&self) {
        self.send(MainMessage::UserSpawned(UserSpawnedMessage {
            user_info: self.user_info.clone(),
        }));
    }

    pub(crate) fn add_task_executed(&self) {
        self.send(MainMessage::TaskExecuted(TaskExecutedMessage {
            user_id: self.user_info.id,
        }));
    }

    pub(crate) fn add_user_self_stopped(&self) {
        self.send(MainMessage::UserSelfStopped(UserSelfStoppedMessage {
            user_id: self.user_info.id,
        }));
    }

    pub fn get_user_id(&self) -> u64 {
        self.user_info.id
    }
}
