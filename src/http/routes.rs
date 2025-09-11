use std::collections::HashMap;

use crate::http::{
    request::{HttpMethod, HttpRequest, HttpVersion},
    response::{HttpResponse, HttpStatus},
};

pub struct Route {
    method: HttpMethod,
    path: String, // /echo/{text}
    handler: fn(request: &HttpRequest, params: &HashMap<String, String>) -> HttpResponse,
}

pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    pub fn new() -> Self {
        Router { routes: Vec::new() }
    }

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

    pub fn route(&self, request: &HttpRequest) -> HttpResponse {
        for route in &self.routes {
            if route.method == request.method {
                let route_path = route.path.split('/').collect::<Vec<&str>>();
                let request_path = request.path.split('/').collect::<Vec<&str>>();

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

        HttpResponse {
            version: HttpVersion::Http1_1.to_string(),
            status: HttpStatus::NotFound,
            headers: HashMap::new(),
            body: Some("404 Not Found".to_string()),
        }
    }
}

pub fn echo_handler(request: &HttpRequest, params: &HashMap<String, String>) -> HttpResponse {
    let text = params.get("text").map(|s| s.as_str()).unwrap_or("");
    let body = text.to_string();
    let headers = HashMap::from([
        ("Content-Length".to_string(), body.len().to_string()),
        ("Connection".to_string(), "Close".to_string()),
        ("Content-Type".to_string(), "text/plain".to_string()),
    ]);

    HttpResponse {
        version: HttpVersion::Http1_1.to_string(),
        status: HttpStatus::Ok,
        headers,
        body: Some(body),
    }
}
