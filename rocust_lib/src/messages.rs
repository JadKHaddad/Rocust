use crate::{results::EndpointTypeName, tasks::EventsTaskInfo, test::user::EventsUserInfo};

pub enum MainMessage {
    ResultMessage(ResultMessage),
    UserSpawned(UserSpawnedMessage),
    UserSelfStopped(UserSelfStoppedMessage),
    UserFinished(UserFinishedMessage),
    UserPanicked(UserPanickedMessage),
    UserUnknownStatus(UserUnknownStatusMessage),
    TaskExecuted(TaskExecutedMessage),
}

pub struct UserSelfStoppedMessage {
    pub(crate) user_info: EventsUserInfo,
}

pub struct UserFinishedMessage {
    pub(crate) user_info: EventsUserInfo,
}

pub struct UserPanickedMessage {
    pub(crate) user_info: EventsUserInfo,
    pub(crate) _error: String,
}

pub struct UserUnknownStatusMessage {
    pub(crate) user_info: EventsUserInfo,
}

pub struct TaskExecutedMessage {
    pub(crate) user_info: EventsUserInfo,
    pub(crate) task_info: EventsTaskInfo,
}

pub struct UserSpawnedMessage {
    pub(crate) user_info: EventsUserInfo,
}

pub enum ResultMessage {
    Success(SuccessResultMessage),
    Failure(FailureResultMessage),
    Error(ErrorResultMessage),
}

pub struct SuccessResultMessage {
    pub(crate) user_info: EventsUserInfo,
    pub(crate) endpoint_type_name: EndpointTypeName,
    pub(crate) response_time: f64,
}

pub struct FailureResultMessage {
    pub(crate) user_info: EventsUserInfo,
    pub(crate) endpoint_type_name: EndpointTypeName,
}

pub struct ErrorResultMessage {
    pub(crate) user_info: EventsUserInfo,
    pub(crate) endpoint_type_name: EndpointTypeName,
    pub(crate) error: String,
}
