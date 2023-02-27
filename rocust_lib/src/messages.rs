use crate::{results::EndpointTypeName, user::EventsUserInfo};

pub enum MainMessage {
    ResultMessage(ResultMessage),
    UserSpawned(UserSpawnedMessage),
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
    pub(crate) user_id: u64,
    pub(crate) endpoint_type_name: EndpointTypeName,
    pub(crate) response_time: f64,
}

pub struct FailureResultMessage {
    pub(crate) user_id: u64,
    pub(crate) endpoint_type_name: EndpointTypeName,
}

pub struct ErrorResultMessage {
    pub(crate) user_id: u64,
    pub(crate) endpoint_type_name: EndpointTypeName,
    pub(crate) error: String,
}
