use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub id: u64,
    pub name: String,
}

impl UserInfo {
    pub fn new(id: u64, name: String) -> Self {
        Self { id, name }
    }
}

pub struct SpawnedUserInfo {
    pub id: u64,
    pub name: String,
    pub total_tasks: u64,
}

impl SpawnedUserInfo {
    pub fn new(id: u64, name: String, total_tasks: u64) -> Self {
        Self {
            id,
            name,
            total_tasks,
        }
    }
}

pub struct SpawnedUserPanicInfo {
    pub id: u64,
    pub name: String,
}

impl SpawnedUserPanicInfo {
    pub fn new(id: u64, name: String) -> Self {
        Self { id, name }
    }
}

pub struct UserController {
    id: u64,
    token: Arc<CancellationToken>,
}

impl UserController {
    pub fn new(id: u64, token: Arc<CancellationToken>) -> Self {
        Self { id, token }
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }

    pub fn stop(&self) {
        self.token.cancel();
    }
}
