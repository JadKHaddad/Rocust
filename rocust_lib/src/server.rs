use crate::{
    prometheus_exporter::PrometheusExporter,
    results::{AllResults, SerAllResults},
    test::controller::TestController,
};
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use hyper::{Error as HyperError, StatusCode};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;

#[derive(Clone)]
struct ServerState {
    test_controller: TestController,
    all_results_arc_rwlock: Arc<RwLock<AllResults>>,
    prometheus_exporter_arc: Arc<PrometheusExporter>,
}

pub struct Server {
    test_controller: TestController,
    all_results_arc_rwlock: Arc<RwLock<AllResults>>,
    prometheus_exporter_arc: Arc<PrometheusExporter>,
    addr: SocketAddr,
}

impl Server {
    pub(crate) fn new(
        test_controller: TestController,
        all_results_arc_rwlock: Arc<RwLock<AllResults>>,
        prometheus_exporter_arc: Arc<PrometheusExporter>,
        addr: SocketAddr,
    ) -> Self {
        Self {
            test_controller,
            all_results_arc_rwlock,
            prometheus_exporter_arc,
            addr,
        }
    }

    pub async fn run(&self) -> Result<(), HyperError> {
        let app = Router::new()
            .route("/results", get(get_results))
            .route("/metrics", get(metrics))
            .route("/stop", get(stop))
            .with_state(ServerState {
                test_controller: self.test_controller.clone(),
                all_results_arc_rwlock: self.all_results_arc_rwlock.clone(),
                prometheus_exporter_arc: self.prometheus_exporter_arc.clone(),
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

async fn metrics(State(server_state): State<ServerState>) -> impl IntoResponse {
    let mut response: Response<String> = Response::default();

    match server_state.prometheus_exporter_arc.get_metrics() {
        Ok(metrics) => {
            *response.status_mut() = StatusCode::OK;
            response.body_mut().push_str(&metrics);
        }
        Err(e) => {
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            response.body_mut().push_str(&format!("Error: {}", e));
        }
    }

    response
}
