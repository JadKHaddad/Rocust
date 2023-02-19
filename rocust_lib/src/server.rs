use crate::test::TestController;
use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use hyper::{Error as HyperError, StatusCode};
use std::net::SocketAddr;

pub struct Server {
    test_controller: TestController,
    addr: SocketAddr,
}

impl Server {
    pub fn new(test_controller: TestController, addr: SocketAddr) -> Self {
        Self {
            test_controller,
            addr,
        }
    }

    pub async fn run(&self) -> Result<(), HyperError> {
        let app = Router::new()
            .route("/get_results", get(get_results))
            .route("/stop", get(stop))
            .with_state(self.test_controller.clone());
        axum::Server::bind(&self.addr)
            .serve(app.into_make_service())
            .with_graceful_shutdown(self.test_controller.token.cancelled())
            .await
    }
}

async fn stop(State(test_controller): State<TestController>) -> impl IntoResponse {
    test_controller.stop();
    StatusCode::OK
}

async fn get_results(State(test_controller): State<TestController>) -> impl IntoResponse {
    let results = test_controller.get_results().await;
    Json(results)
}
