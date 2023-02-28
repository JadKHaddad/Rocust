use crate::results::{AllResults, EndpointTypeName};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct UserStatsCollection {
    user_stats_map: HashMap<u64, UserStats>,
}

impl UserStatsCollection {
    pub fn new() -> Self {
        Self {
            user_stats_map: HashMap::new(),
        }
    }

    pub(crate) fn add_success(
        &mut self,
        user_id: &u64,
        endpoint_type_name: &EndpointTypeName,
        response_time: f64,
    ) {
        if let Some(user_stats) = self.user_stats_map.get_mut(user_id) {
            user_stats
                .all_results
                .add_success(endpoint_type_name, response_time);
        }
    }

    pub(crate) fn insert_user(&mut self, user_id: u64, user_name: String) {
        self.user_stats_map.insert(
            user_id,
            UserStats::new(
                UserStatsInfo::new(user_id, user_name),
                AllResults::default(),
            ),
        );
    }

    pub(crate) fn add_failure(&mut self, user_id: &u64, endpoint_type_name: &EndpointTypeName) {
        if let Some(user_stats) = self.user_stats_map.get_mut(user_id) {
            user_stats.all_results.add_failure(endpoint_type_name);
        }
    }

    pub(crate) fn add_error(
        &mut self,
        user_id: &u64,
        endpoint_type_name: &EndpointTypeName,
        error: &String,
    ) {
        if let Some(user_stats) = self.user_stats_map.get_mut(user_id) {
            user_stats.all_results.add_error(endpoint_type_name, error);
        }
    }

    pub(crate) fn calculate_per_second(&mut self, elapsed: &Duration) {
        for user_stats in self.user_stats_map.values_mut() {
            user_stats.all_results.calculate_per_second(elapsed);
        }
    }

    pub(crate) fn set_user_status(&mut self, user_id: &u64, status: UserStatus) {
        // if status is cancelled, we don't want to overwrite it with other statuses
        if let Some(user_stats) = self.user_stats_map.get_mut(user_id) {
            if user_stats.user_info.status == UserStatus::Cancelled {
                return;
            }
            user_stats.user_info.status = status;
        }
    }

    pub(crate) fn increment_total_tasks(&mut self, user_id: &u64) {
        if let Some(user_stats) = self.user_stats_map.get_mut(user_id) {
            user_stats.user_info.total_tasks += 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserStats {
    pub user_info: UserStatsInfo,
    pub all_results: AllResults,
}

impl UserStats {
    pub fn new(user_info: UserStatsInfo, all_results: AllResults) -> Self {
        Self {
            user_info,
            all_results,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UserStatus {
    Finished,
    Spawned,
    Panicked,
    Cancelled, // cancelled by himself
    Unknown,
}

#[derive(Debug, Clone)]
pub struct UserStatsInfo {
    pub id: u64,
    pub name: String,
    pub status: UserStatus,
    pub total_tasks: u64,
}

impl UserStatsInfo {
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id,
            name,
            status: UserStatus::Spawned,
            total_tasks: 0,
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
