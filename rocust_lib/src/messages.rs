use crate::results::EndpointTypeName;

pub enum MainMessage {
    ResultMessage(ResultMessage),
    UserSpawned(UserSpawnedMessage),
}

pub struct UserSpawnedMessage {
    pub id: u64,
    pub name: String,
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
