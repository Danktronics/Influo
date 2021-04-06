use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use hyper::service::{make_service_fn, service_fn};
use hyper::body::Buf;
use hyper::{Server, Request, Response, header, Body, Method, StatusCode};
use route_recognizer::{Router, Params};

use anyhow::anyhow;

use crate::model::{Configuration, project::Project};
use crate::model::channel::ProcedureConnection;
use crate::filesystem::{write_configuration};

enum RouterRoute {
    Projects, // /projects
    Project // /projects/:project_id
}

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

async fn api_get_project(_request: Request<Body>, url_params: &Params, configuration: Arc<Mutex<Configuration>>) -> Result<Response<Body>, anyhow::Error> {
    let configuration = &*configuration.lock().unwrap();

    match base64_decode(url_params.find("project_url").unwrap()) {
        Ok(project_url) => {
            match configuration.projects.iter().find(|p| p.url == project_url) {
                Some(project) => Ok(create_api_response_body(StatusCode::OK, Body::from(serde_json::to_string(&project)?))),
                None => Ok(create_api_response_body(StatusCode::NOT_FOUND, r#"{"error": "Unknown Project"}"#.into()))
            }
        },
        Err(_error) => Ok(create_api_response_body(StatusCode::BAD_REQUEST, r#"{"error": "Invalid Base 64 Project URL"}"#.into()))
    }
}

async fn api_delete_project(_request: Request<Body>, url_params: &Params, configuration: Arc<Mutex<Configuration>>, procedure_connections: Arc<Mutex<Vec<ProcedureConnection>>>) -> Result<Response<Body>, anyhow::Error> {
    let configuration = &*configuration.lock().unwrap();

    match base64_decode(url_params.find("project_url").unwrap()) {
        Ok(project_url) => {
            match configuration.projects.iter().find(|p| p.url == project_url) {
                Some(project) => {
                    unimplemented!()
                },
                None => Ok(create_api_response_body(StatusCode::NOT_FOUND, r#"{"error": "Unknown Project"}"#.into()))
            }
        },
        Err(_error) => Ok(create_api_response_body(StatusCode::BAD_REQUEST, r#"{"error": "Invalid Base 64 Project URL"}"#.into()))
    }
}

async fn root_handle_request(request: Request<Body>, router: Arc<Router<RouterRoute>>, configuration: Arc<Mutex<Configuration>>, procedure_connections: Arc<Mutex<Vec<ProcedureConnection>>>) -> Result<Response<Body>, anyhow::Error> {
    debug!(format!("Request received: {} \"{}\"", request.method(), request.uri().path()));

    match router.recognize(request.uri().path()) {
        Ok(route_match) => {
            match route_match.handler() {
                RouterRoute::Projects => {
                    match *request.method() {
                        Method::GET => api_get_projects(request, configuration).await,
                        Method::POST => api_create_project(request, configuration).await,
                        _ => Ok(create_api_response_body(StatusCode::NOT_FOUND, r#"{"error": "Not Found"}"#.into()))
                    }
                },
                RouterRoute::Project => {
                    match *request.method() {
                        Method::GET => api_get_project(request, route_match.params(), configuration).await,
                        Method::DELETE => api_delete_project(request, route_match.params(), configuration, procedure_connections).await,
                        _ => Ok(create_api_response_body(StatusCode::NOT_FOUND, r#"{"error": "Not Found"}"#.into()))
                    }
                }
            }
        },
        Err(_) => Ok(create_api_response_body(StatusCode::NOT_FOUND, r#"{"error": "Not Found"}"#.into()))
    }
}

pub fn start_http_server(configuration: Arc<Mutex<Configuration>>, procedure_connections: Arc<Mutex<Vec<ProcedureConnection>>>) -> Result<(), anyhow::Error> {
    let port;
    match &configuration.lock().unwrap().api {
        Some(api) => match &api.http {
            Some(http) => port = http.port,
            None => return Err(anyhow!("Missing HTTP configuration"))
        },
        None => return Err(anyhow!("Missing API configuration"))
    }
    
    debug!(format!("Attempting to bind HTTP server to 127.0.0.1:{}", port));
    let server_builder = Server::try_bind(&SocketAddr::from(([127, 0, 0, 1], port)))?;
    info!(format!("Bound HTTP server to 127.0.0.1:{}", port));

    let mut router = Router::new();
    router.add("/projects", RouterRoute::Projects);
    router.add("/projects/:project_url", RouterRoute::Project);
    let router = Arc::new(router);

    let service = make_service_fn(move |_| {
        let configuration = Arc::clone(&configuration);
        let procedure_connections = Arc::clone(&procedure_connections);
        let router = Arc::clone(&router);

        async {
            Ok::<_, anyhow::Error>(service_fn(move |request| {
                root_handle_request(request, Arc::clone(&router), Arc::clone(&configuration), Arc::clone(&procedure_connections))
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

#[inline]
fn base64_encode(data: &str) -> String {
    base64::encode_config(&data, base64::URL_SAFE_NO_PAD)
}

#[inline]
fn base64_decode(data: &str) -> Result<String, anyhow::Error> {
    Ok(String::from_utf8(base64::decode_config(&data, base64::URL_SAFE_NO_PAD)?)?)
}