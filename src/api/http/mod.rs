use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use hyper::service::{make_service_fn, service_fn};
use hyper::{Server, Request, Response, header, Body, Method, StatusCode};

use crate::model::project::Project;

async fn api_get_projects(_request: Request<Body>, projects: Arc<Mutex<Vec<Project>>>) -> Result<Response<Body>, anyhow::Error> {
    let projects = &*projects.lock().unwrap();

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(projects)?))
        .unwrap())
}

async fn root_handle_request(request: Request<Body>, projects: Arc<Mutex<Vec<Project>>>) -> Result<Response<Body>, anyhow::Error> {
    debug!(format!("Request received: {} \"{}\"", request.method(), request.uri().path()));

    match (request.method(), request.uri().path()) {
        (&Method::GET, "/projects") => api_get_projects(request, projects).await,
        _ => {
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("Not found".into())
                .unwrap()
            )
        }
    }
}

pub fn start_http_server(projects: Arc<Mutex<Vec<Project>>>) -> Result<(), anyhow::Error> {
    let server_builder = Server::try_bind(&SocketAddr::from(([127, 0, 0, 1], 4200)))?;

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