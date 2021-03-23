use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use hyper::service::{make_service_fn, service_fn};
use hyper::{Server, Request, Response, header, Body, Method, StatusCode};

use crate::model::project::Project;
use crate::filesystem::{read_configuration, write_configuration};

fn create_api_response(status: StatusCode, body: Body) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .unwrap()
}

async fn api_get_projects(_request: Request<Body>, projects: Arc<Mutex<Vec<Project>>>) -> Result<Response<Body>, anyhow::Error> {
    let projects = &*projects.lock().unwrap();

    Ok(create_api_response(StatusCode::OK, Body::from(serde_json::to_string(projects)?)))
}

async fn api_create_project(request: Request<Body>, projects: Arc<Mutex<Vec<Project>>>) -> Result<Response<Body>, anyhow::Error> {
    let body = hyper::body::aggregate(request).await?;
    let data = serde_json::from_reader(body.reader())?;

    let persistent = match data.get("persistent") {
        Some(value) => match value.as_bool() {
            Some(persistent) => persistent,
            None => return Err(create_api_response(StatusCode::BAD_REQUEST, Body::from(r#"{"error": "persistent must be a boolean""#.into())))
        },
        None => false
    };

    if persistent {
        let mut configuration = read_configuration();
        match configuration.get_mut("projects") {
            Some(raw_projects) => match raw_projects.as_array_mut() {
                Some(projects) => projects.push()
            },
            None => {
                return Err(create_api_response(StatusCode::INTERNAL_SERVER_ERROR, r#"{"error": "Configuration missing projects array"}"#))
            }
        }
    }

    let projects = &*projects.lock().unwrap();
}

async fn root_handle_request(request: Request<Body>, projects: Arc<Mutex<Vec<Project>>>) -> Result<Response<Body>, anyhow::Error> {
    debug!(format!("Request received: {} \"{}\"", request.method(), request.uri().path()));

    match (request.method(), request.uri().path()) {
        (&Method::GET, "/projects") => api_get_projects(request, projects).await,
        (&Method::POST, "/projects") => api_create_project(request, projects).await,
        _ => {
            Ok(create_api_response(StatusCode::NOT_FOUND, r#"{"error": "Not Found"}"#.into()))
        }
    }
}

pub fn start_http_server(port: u16, projects: Arc<Mutex<Vec<Project>>>) -> Result<(), anyhow::Error> {
    let server_builder = Server::try_bind(&SocketAddr::from(([127, 0, 0, 1], port)))?;

    let service = make_service_fn(move |_| {
        let projects = Arc::clone(&projects);

        async {
            Ok::<_, anyhow::Error>(service_fn(move |request| {
                root_handle_request(request, Arc::clone(&projects))
            }))
        }
    });

    tokio::spawn(async move {
        let server = server_builder.serve(service);
        if let Err(error) = server.await {
            eprintln!("{:?}", error);
        }
    });
    
    Ok(())
}