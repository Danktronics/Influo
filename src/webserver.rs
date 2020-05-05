use hyper::{header, Body, Client, Method, Request, Response, Server, StatusCode};

async fn get_status() -> Result<Response<Body>> {
    Ok(Response::new("{}"))
}

async fn handle_request(request: Request<Body>, client: Client<HttpConnector>) -> Result<Response<Body>> {
    match (request.method(), request.uri().path()) {
        (&Method::GET "/") => Ok(Response::new("Welcome to Influo")),
        (&Method::GET, "/api") => get_status().await,
        _ => {
            Ok(Response::builder().status(StatusCode::NOT_FOUND).body("404 Not Found").unwrap())
        }
    }
}

async fn start_webserver(port: u16) -> Result<()> {
    let client: Client = Client::new();
    let address = format!("127.0.0.1:{}", port);

    let service = make_service_fn(move |_| {
        let client: Client = client.clone();
        async {
            Ok::<_, GenericError>(service_fn(move |request| {
                handle_request(request, client.to_owned());
            }))
        }
    });

    let webserver = Server::bind(&address).serve(service);
    info!(format!("Webserver is listening on http://{}", address));
    webserver.await?; // Keep Alive
    Ok(());
}