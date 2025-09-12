use std::collections::HashMap;

use crate::http::{
    request::{HttpMethod, HttpRequest, HttpVersion},
    response::{HttpResponse, HttpStatusCode, StatusLine},
};

/// Represents a single route
pub struct Route {
    method: HttpMethod,
    path: String, // /echo/{text}
    handler: fn(request: &HttpRequest, params: &HashMap<String, String>) -> HttpResponse,
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

        router
    }

    /// Registers a GET route
    pub fn get(
        &mut self,
        path: &str,
        handler: fn(&HttpRequest, &HashMap<String, String>) -> HttpResponse,
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
    pub fn route(&self, request: &HttpRequest) -> HttpResponse {
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
                        return (route.handler)(request, &params);
                    }
                }
            }
        }

        let status_line = StatusLine {
            version: HttpVersion::Http1_1.to_string(),
            status: HttpStatusCode::NotFound,
        };

        HttpResponse {
            status_line,
            headers: HashMap::new(),
            body: Some("404 Not Found".to_string()),
        }
    }
}

/// Handler that handles root path
pub fn root_handler(_request: &HttpRequest, _params: &HashMap<String, String>) -> HttpResponse {
    let body = "Welcome to the Rust HTTP Server!".to_string();
    let headers = HashMap::from([
        ("Content-Length".to_string(), body.len().to_string()),
        ("Content-Type".to_string(), "text/plain".to_string()),
        ("Connection".to_string(), "Close".to_string()),
    ]);

    let status_line = StatusLine {
        version: HttpVersion::Http1_1.to_string(),
        status: HttpStatusCode::Ok,
    };

    HttpResponse {
        status_line,
        headers,
        body: Some(body),
    }
}

/// Handler that echoes text parameter
pub fn echo_handler(_request: &HttpRequest, params: &HashMap<String, String>) -> HttpResponse {
    let text = params.get("text").map(|s| s.as_str()).unwrap_or("");
    let body = text.to_string();
    let headers = HashMap::from([
        ("Content-Length".to_string(), body.len().to_string()),
        ("Content-Type".to_string(), "text/plain".to_string()),
        ("Connection".to_string(), "Close".to_string()),
    ]);

    let status_line = StatusLine {
        version: HttpVersion::Http1_1.to_string(),
        status: HttpStatusCode::Ok,
    };

    HttpResponse {
        status_line,
        headers,
        body: Some(body),
    }
}
