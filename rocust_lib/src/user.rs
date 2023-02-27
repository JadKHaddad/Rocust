use crate::results::AllResults;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct UserSummary {
    pub user_info: UserSummaryInfo,
    pub all_results: AllResults,
}

impl UserSummary {
    pub fn new(user_info: UserSummaryInfo, all_results: AllResults) -> Self {
        Self {
            user_info,
            all_results,
        }
    }
}

#[derive(Debug, Clone)]
pub enum UserSummaryStatus {
    Joined,
    Spawned,
    Panicked,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct UserSummaryInfo {
    pub id: u64,
    pub name: String,
    pub status: UserSummaryStatus,
    pub total_tasks: Option<u64>, // None if panicked
}

impl UserSummaryInfo {
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id,
            name,
            status: UserSummaryStatus::Spawned,
            total_tasks: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EventsUserInfo {
    pub id: u64,
    pub name: String,
}

impl EventsUserInfo {
    pub fn new(id: u64, name: String) -> Self {
        Self { id, name }
    }
}

pub struct JoinHandleSpawnedUserInfo {
    pub id: u64,
    pub name: String,
    pub total_tasks: u64,
}

impl JoinHandleSpawnedUserInfo {
    pub fn new(id: u64, name: String, total_tasks: u64) -> Self {
        Self {
            id,
            name,
            total_tasks,
        }
    }
}

pub struct JoinHandleSpawnedUserPanicInfo {
    pub id: u64,
    pub name: String,
}

impl JoinHandleSpawnedUserPanicInfo {
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
