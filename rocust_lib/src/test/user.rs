pub mod context;

use crate::results::{AllResults, EndpointTypeName, SummaryAllResults};
use serde::Serialize;
use serde_json::Error as SerdeJsonError;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize)]
struct SummaryUserStatsCollection {
    user_stats_vec: Vec<SummaryUserStats>,
}

#[derive(Debug, Clone, Serialize)]
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

    pub(crate) fn json_string(&self) -> Result<String, SerdeJsonError> {
        let summary_user_stats_collection: SummaryUserStatsCollection = self.clone().into();
        serde_json::to_string(&summary_user_stats_collection.user_stats_vec)
    }
}

impl Into<SummaryUserStatsCollection> for UserStatsCollection {
    fn into(self) -> SummaryUserStatsCollection {
        let user_stats_vec = self
            .user_stats_map
            .into_iter()
            .map(|(_, user_stats)| user_stats.into())
            .collect();
        SummaryUserStatsCollection { user_stats_vec }
    }
}
#[derive(Debug, Clone, Serialize)]
struct SummaryUserStats {
    user_info: UserStatsInfo,
    all_results: SummaryAllResults,
}

#[derive(Debug, Clone, Serialize)]
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

impl Into<SummaryUserStats> for UserStats {
    fn into(self) -> SummaryUserStats {
        SummaryUserStats {
            user_info: self.user_info,
            all_results: self.all_results.into(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum UserStatus {
    Finished,
    Spawned,
    Panicked,
    Cancelled, // cancelled by himself
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
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
    token: Arc<CancellationToken>,
}

impl UserController {
    pub fn new(token: Arc<CancellationToken>) -> Self {
        Self { token }
    }

    pub fn stop(&self) {
        self.token.cancel();
    }
}
