use parking_lot::RwLock;
use poem::{
    get, handler,
    listener::TcpListener,
    middleware::AddData,
    web::{Data, Path},
    EndpointExt, Route, Server,
};
use rocust_lib::{test::Test, EndPoint, Runnable, Status};
use std::collections::HashMap;
use std::sync::Arc;

#[handler]
fn add_test(Path(id): Path<String>, tests: Data<&TestCollection>) -> String {
    let mut tests = tests.write();

    let new_test = Test::new(
        id.clone(),
        300,
        Some(50),
        (3, 10),
        "https://google.com".to_string(),
        vec![
            EndPoint::new_get(
                "/".to_string(),
                None,
                Some(vec![(String::from("id"), String::from("6"))]),
            ),
            EndPoint::new_get("/get".to_string(), None, None),
            EndPoint::new_post(
                "/post".to_string(),
                None,
                Some(String::from("this is body")),
            ),
            EndPoint::new_put("/put".to_string(), None, None),
            EndPoint::new_delete("/delete".to_string(), None),
        ],
        None,
        format!("log/{}.log", id),
        false,
        false,
    );

    tests.insert(id.clone(), new_test);

    format!("Ok")
}

#[handler]
fn start_test(Path(id): Path<String>, tests: Data<&TestCollection>) -> String {
    let tests = tests.read();
    if let Some(test) = tests.get(&id) {
        let mut test = test.clone();
        tokio::spawn(async move {
            test.run().await;
        });
        format!("Ok")
    } else {
        format!("NotFound")
    }
}

#[handler]
fn stop_test(Path(id): Path<String>, tests: Data<&TestCollection>) -> String {
    let tests = tests.read();
    if let Some(test) = tests.get(&id) {
        test.stop();
        format!("Ok")
    } else {
        format!("NotFound")
    }
}

#[handler]
fn index(tests: Data<&TestCollection>) -> String {
    let tests = tests.read();
    let tests: Vec<(String, Status)> = tests
        .clone()
        .into_iter()
        .map(|(id, test)| (id, test.get_status()))
        .collect();
    format!("{:?}", tests)
}

type TestCollection = Arc<RwLock<HashMap<String, Test>>>;
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let tests: TestCollection = Arc::new(RwLock::new(HashMap::new()));
    let app = Route::new()
        .at("/", get(index))
        .at("/add/:id", get(add_test))
        .at("/start/:id", get(start_test))
        .at("/stop/:id", get(stop_test))
        .with(AddData::new(tests));
    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
