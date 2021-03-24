use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use hyper::service::{make_service_fn, service_fn};
use hyper::body::Buf;
use hyper::{Server, Request, Response, header, Body, Method, StatusCode};

use serde::Deserialize;

use anyhow::anyhow;

use crate::model::{Configuration, project::Project};
use crate::filesystem::{write_configuration};

fn create_api_response(status: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(Body::empty())
        .unwrap()
}

fn create_api_response_body(status: StatusCode, body: Body) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .unwrap()
}

async fn api_get_projects(_request: Request<Body>, configuration: Arc<Mutex<Configuration>>) -> Result<Response<Body>, anyhow::Error> {
    let configuration = &*configuration.lock().unwrap();

    Ok(create_api_response_body(StatusCode::OK, Body::from(serde_json::to_string(&configuration.projects)?)))
}

async fn api_create_project(request: Request<Body>, configuration: Arc<Mutex<Configuration>>) -> Result<Response<Body>, anyhow::Error> {
    let body = hyper::body::aggregate(request).await?;
    let project: Project = serde_json::from_reader(body.reader())?;
    let configuration = &mut *configuration.lock().unwrap();

    if project.persistent {
        let mut persistent_project = project.clone();
        persistent_project.procedures.retain(|p| p.persistent);
        write_configuration(&serde_json::to_value(persistent_project)?)?;
    }

    configuration.projects.push(project);
    Ok(create_api_response(StatusCode::OK))
}

async fn root_handle_request(request: Request<Body>, configuration: Arc<Mutex<Configuration>>) -> Result<Response<Body>, anyhow::Error> {
    debug!(format!("Request received: {} \"{}\"", request.method(), request.uri().path()));

    match (request.method(), request.uri().path()) {
        (&Method::GET, "/projects") => api_get_projects(request, configuration).await,
        (&Method::POST, "/projects") => api_create_project(request, configuration).await,
        _ => {
            Ok(create_api_response_body(StatusCode::NOT_FOUND, r#"{"error": "Not Found"}"#.into()))
        }
    }
}

pub fn start_http_server(configuration: Arc<Mutex<Configuration>>) -> Result<(), anyhow::Error> {
    let port;
    match &configuration.lock().unwrap().api {
        Some(api) => match &api.http {
            Some(http) => port = http.port,
            None => return Err(anyhow!("Missing HTTP configuration"))
        },
        None => return Err(anyhow!("Missing API configuration"))
    }
    
    let server_builder = Server::try_bind(&SocketAddr::from(([127, 0, 0, 1], port)))?;

    let service = make_service_fn(move |_| {
        let configuration = Arc::clone(&configuration);

        async {
            Ok::<_, anyhow::Error>(service_fn(move |request| {
                root_handle_request(request, Arc::clone(&configuration))
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