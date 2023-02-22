use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub struct UserInfo {
    pub id: u64,
    pub name: String,
    pub total_tasks: u64,
}

impl UserInfo {
    pub fn new(id: u64, name: String, total_tasks: u64) -> Self {
        Self {
            id,
            name,
            total_tasks,
        }
    }
}

pub struct UserPanicInfo {
    pub id: u64,
    pub name: String,
}

impl UserPanicInfo {
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
