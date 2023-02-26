use crate::{results::EndpointTypeName, user::UserInfo};

pub enum MainMessage {
    ResultMessage(ResultMessage),
    UserSpawned(UserSpawnedMessage),
}

pub struct UserSpawnedMessage {
    pub(crate) user_info: UserInfo,
}

pub enum ResultMessage {
    Success(SuccessResultMessage),
    Failure(FailureResultMessage),
    Error(ErrorResultMessage),
}

pub struct SuccessResultMessage {
    pub(crate) user_info: UserInfo,
    pub(crate) endpoint_type_name: EndpointTypeName,
    pub(crate) response_time: f64,
}

pub struct FailureResultMessage {
    pub(crate) user_info: UserInfo,
    pub(crate) endpoint_type_name: EndpointTypeName,
}

pub struct ErrorResultMessage {
    pub(crate) user_info: UserInfo,
    pub(crate) endpoint_type_name: EndpointTypeName,
    pub(crate) error: String,
}
