use crate::{
    results::{AllResults, SerAllResults},
    test::controller::TestController,
};
use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use hyper::{Error as HyperError, StatusCode};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;

#[derive(Clone)]
struct ServerState {
    test_controller: TestController,
    all_results_arc_rwlock: Arc<RwLock<AllResults>>,
}

pub struct Server {
    test_controller: TestController,
    all_results_arc_rwlock: Arc<RwLock<AllResults>>,
    addr: SocketAddr,
}

impl Server {
    pub fn new(
        test_controller: TestController,
        all_results_arc_rwlock: Arc<RwLock<AllResults>>,
        addr: SocketAddr,
    ) -> Self {
        Self {
            test_controller,
            all_results_arc_rwlock,
            addr,
        }
    }

    pub async fn run(&self) -> Result<(), HyperError> {
        let app = Router::new()
            .route("/results", get(get_results))
            .route("/stop", get(stop))
            .with_state(ServerState {
                test_controller: self.test_controller.clone(),
                all_results_arc_rwlock: self.all_results_arc_rwlock.clone(),
            });
        axum::Server::bind(&self.addr)
            .serve(app.into_make_service())
            .with_graceful_shutdown(self.test_controller.cancelled())
            .await
    }
}

async fn stop(State(server_state): State<ServerState>) -> impl IntoResponse {
    server_state.test_controller.stop();
    StatusCode::OK
}

async fn get_results(State(server_state): State<ServerState>) -> impl IntoResponse {
    let ser_all_results: SerAllResults = server_state
        .all_results_arc_rwlock
        .read()
        .await
        .clone()
        .into();
    Json(ser_all_results)
}
