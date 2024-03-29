use crate::{
    messages::{
        ErrorResultMessage, FailureResultMessage, MainMessage, ResultMessage, SuccessResultMessage,
        TaskExecutedMessage, UserFinishedMessage, UserPanickedMessage, UserSelfStoppedMessage,
        UserSpawnedMessage, UserUnknownStatusMessage,
    },
    results::EndpointTypeName,
    tasks::EventsTaskInfo,
    test::user::EventsUserInfo,
};
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct EventsHandler {
    user_info: EventsUserInfo,
    sender: Sender<MainMessage>,
}

impl EventsHandler {
    pub(crate) fn new(user_info: EventsUserInfo, sender: Sender<MainMessage>) -> Self {
        Self { user_info, sender }
    }

    async fn send(&self, message: MainMessage) {
        let _ = self.sender.send(message).await;
    }

    pub async fn add_success(&self, r#type: String, name: String, response_time: f64) {
        self.send(MainMessage::ResultMessage(ResultMessage::Success(
            SuccessResultMessage {
                user_info: self.user_info.clone(),
                endpoint_type_name: EndpointTypeName { r#type, name },
                response_time,
            },
        )))
        .await;
    }

    pub async fn add_failure(&self, r#type: String, name: String) {
        self.send(MainMessage::ResultMessage(ResultMessage::Failure(
            FailureResultMessage {
                user_info: self.user_info.clone(),
                endpoint_type_name: EndpointTypeName { r#type, name },
            },
        )))
        .await;
    }

    pub async fn add_error(&self, r#type: String, name: String, error: String) {
        self.send(MainMessage::ResultMessage(ResultMessage::Error(
            ErrorResultMessage {
                user_info: self.user_info.clone(),
                endpoint_type_name: EndpointTypeName { r#type, name },
                error,
            },
        )))
        .await;
    }

    pub(crate) async fn add_user_spawned(&self) {
        self.send(MainMessage::UserSpawned(UserSpawnedMessage {
            user_info: self.user_info.clone(),
        }))
        .await;
    }

    pub(crate) async fn add_task_executed(&self, task_info: EventsTaskInfo) {
        self.send(MainMessage::TaskExecuted(TaskExecutedMessage {
            user_info: self.user_info.clone(),
            task_info,
        }))
        .await;
    }

    pub(crate) async fn add_user_self_stopped(&self) {
        self.send(MainMessage::UserSelfStopped(UserSelfStoppedMessage {
            user_info: self.user_info.clone(),
        }))
        .await;
    }

    pub(crate) async fn add_user_finished(&self) {
        self.send(MainMessage::UserFinished(UserFinishedMessage {
            user_info: self.user_info.clone(),
        }))
        .await;
    }

    pub(crate) async fn add_user_panicked(&self, error: String) {
        self.send(MainMessage::UserPanicked(UserPanickedMessage {
            user_info: self.user_info.clone(),
            _error: error,
        }))
        .await;
    }

    pub(crate) async fn add_user_unknown_status(&self) {
        self.send(MainMessage::UserUnknownStatus(UserUnknownStatusMessage {
            user_info: self.user_info.clone(),
        }))
        .await;
    }
    pub fn get_user_id(&self) -> u64 {
        self.user_info.id
    }
}
