use hyper::client::HttpConnector;
use hyper::service::{make_service_fn, service_fn};
use hyper::{header, Body, Client, Method, Request, Response, Server, StatusCode};
use failure::Error;

async fn get_status() -> Result<Response<&Body>, Error> {
    Ok(Response::builder().status(StatusCode::OK).body(Body::from("{}"))).unwrap())
}

async fn handle_request(request: Request<Body>, client: Client<HttpConnector>) -> Result<Response<Body>, Error> {
    match (request.method(), request.uri().path()) {
        (&Method::GET, "/") => Ok(Response::builder().status(StatusCode::OK).body(Body::from("Welcome to Influo"))).unwrap()),
        (&Method::GET, "/api") => get_status().await,
        _ => {
            Ok(Response::builder().status(StatusCode::NOT_FOUND).body("404 Not Found").unwrap())
        }
    }
}

pub async fn start_webserver(port: u16) -> Result<(), Error> {
    let client = Client::new();
    let address = format!("127.0.0.1:{}", port).parse().unwrap();

    let service = make_service_fn(move |_| {
        let client = client.clone();
        async {
            Ok::<_, Error>(service_fn(move |request| {
                handle_request(request, client.to_owned());
            }))
        }
    });

    let webserver = Server::bind(&address).serve(service);
    info!(format!("Webserver is listening on http://{}", address));
    webserver.await?; // Keep Alive
    Ok(())
}