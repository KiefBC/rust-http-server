use std::{collections::HashMap, net::TcpStream};

use crate::http::{
    request::{HttpMethod, HttpRequest},
    response::HttpStatusCode,
    writer::HttpWriter,
};

/// Represents a single route
pub struct Route {
    method: HttpMethod,
    path: String, // /echo/{text}
    handler: fn(request: &HttpRequest, params: &HashMap<String, String>, stream: &mut TcpStream),
}

/// Manages routes and dispatches requests
pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    /// Creates a new router
    pub fn new() -> Self {
        // default routes
        let mut router = Router { routes: Vec::new() };
        router.get("/", root_handler);
        router.get("/echo/{text}", echo_handler);
        router.get("/user-agent", user_agent_handler);

        router
    }

    /// Registers a GET route
    pub fn get(
        &mut self,
        path: &str,
        handler: fn(&HttpRequest, &HashMap<String, String>, &mut TcpStream),
    ) {
        let route = Route {
            method: HttpMethod::Get,
            path: path.to_string(),
            handler,
        };

        self.routes.push(route);
    }

    /// Finds matching route and executes handler
    /// TODO: We might need to consider changing the "{text}" parsing to be isolated from this
    /// route() since its only for /echo/{text} for now
    pub fn route(&self, request: &HttpRequest, stream: &mut TcpStream) {
        for route in &self.routes {
            if route.method == request.status_line.method {
                let route_path = route.path.split('/').collect::<Vec<&str>>();
                let request_path = request.status_line.path.split('/').collect::<Vec<&str>>();

                if route_path.len() == request_path.len() {
                    let mut params: HashMap<String, String> = HashMap::new();
                    let mut is_match: bool = true;

                    for (i, segment) in route_path.iter().enumerate() {
                        if segment.starts_with('{') && segment.ends_with('}') {
                            let key = segment.trim_start_matches('{').trim_end_matches('}');
                            params.insert(key.to_string(), request_path[i].to_string());
                        } else if segment != &request_path[i] {
                            is_match = false;
                            break;
                        }
                    }

                    if is_match {
                        return (route.handler)(request, &params, stream);
                    }
                }
            }
        }

        let body = "404 Not Found".to_string();

        if let Err(e) = HttpWriter::error_response(stream, HttpStatusCode::NotFound, body) {
            HttpWriter::log_writer_error(e, "router_404");
        }
    }
}

/// Handler that handles root path
pub fn root_handler(
    _request: &HttpRequest,
    _params: &HashMap<String, String>,
    stream: &mut TcpStream,
) {
    let body = "Welcome to the Rust HTTP Server!".to_string();

    if let Err(e) = HttpWriter::ok_response(stream, body) {
        HttpWriter::log_writer_error(e, "root_handler");
    }
}

/// Handler that echoes text parameter
pub fn echo_handler(
    _request: &HttpRequest,
    params: &HashMap<String, String>,
    stream: &mut TcpStream,
) {
    let text = params.get("text").map(|s| s.as_str()).unwrap_or("");
    let body = text.to_string();

    if let Err(e) = HttpWriter::ok_response(stream, body) {
        HttpWriter::log_writer_error(e, "echo_handler");
    }
}

/// Handler that returns User-Agent header
pub fn user_agent_handler(
    request: &HttpRequest,
    _params: &HashMap<String, String>,
    stream: &mut TcpStream,
) {
    let user_agent = request
        .headers
        .get("User-Agent")
        .map(|s| s.as_str())
        .unwrap_or("Unknown");
    let body = user_agent.to_string();

    if let Err(e) = HttpWriter::ok_response(stream, body) {
        HttpWriter::log_writer_error(e, "user_agent_handler");
    }
}
