use std::{collections::HashMap, fs, net::TcpStream};

use crate::http::{
    errors::HttpErrorResponse,
    request::{HttpMethod, HttpRequest},
    response::{HttpResponse, HttpStatusCode, ResponseStatusLine},
    server,
    writer::{send_response, HttpBody, HttpWritable, HttpWriter},
};

/// Represents supported HTTP Encoding types
pub enum HttpEncoding {
    Gzip,
    Deflate,
    Brotli,
}

impl std::fmt::Display for HttpEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let encoding_str = match self {
            HttpEncoding::Gzip => "gzip",
            HttpEncoding::Deflate => "deflate",
            HttpEncoding::Brotli => "brotli",
        };
        write!(f, "{}", encoding_str)
    }
}

impl HttpEncoding {
    // Translates string to HttpEncoding enum
    pub fn from_encoding_string(s: &str) -> Option<HttpEncoding> {
        match s.to_lowercase().as_str() {
            "gzip" => Some(HttpEncoding::Gzip),
            "deflate" => Some(HttpEncoding::Deflate),
            "br" | "brotli" => Some(HttpEncoding::Brotli),
            _ => None,
        }
    }

    // Parses Accept-Encoding header and returns sorted encodings with quality values
    pub fn parse_accept_encoding(header: &str) -> Vec<(HttpEncoding, f32)> {
        // "gzip;q=0.8, deflate;q=0.9, br;q=1.0" -> ["gzip;q=0.8", "deflate;q=0.9", "br;q=1.0"]
        let comma_split = header.split(',').map(str::trim);

        // ["gzip;q=0.8", "deflate;q=0.9", "br;q=1.0"] -> ["gzip", "q=0.8"], ["deflate", "q=0.9"]..
        let semicolon_split =
            comma_split.map(|s| s.split(';').map(str::trim).collect::<Vec<&str>>());

        // "q=0.8" -> "0.8" or "1.0" if not present
        let quality_split = semicolon_split.map(|parts| {
            if parts.is_empty() || parts[0].is_empty() {
                return ("", 0.0);
            }

            let encoding_name = parts[0];

            // if q is present, parse it, else default to 1.0
            let q_value = if parts.len() > 1 && parts[1].starts_with("q=") {
                // (gzip, q=0.8) -> 0.8
                parts[1][2..].parse::<f32>().unwrap_or(1.0)
            } else {
                1.0
            };

            (encoding_name, q_value)
        });

        let mut sorted_quality: Vec<(&str, f32)> =
            quality_split.filter(|(_, q)| *q > 0.0).collect();
        sorted_quality.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let mut encodings: Vec<(HttpEncoding, f32)> = Vec::new();
        for (enc_str, q) in sorted_quality {
            if let Some(enc) = HttpEncoding::from_encoding_string(enc_str) {
                encodings.push((enc, q));
            }
        }

        encodings
    }
}

/// Supports content negotiation for responses
pub trait ContentNegotiable {
    fn for_file(status: HttpStatusCode, filename: &str, content: String) -> Self;
    fn with_negotiation(
        status_code: HttpStatusCode,
        content: String,
        accept_header: Option<&str>,
    ) -> Self;
}

pub struct CompressionMiddleware;

impl CompressionMiddleware {
    // Applies compression based on Accept-Encoding header
    pub fn apply<T: HttpWritable>(
        response: T,
        accept_encoding: Option<&str>,
    ) -> CompressedResponse<T> {
        let encoding = if let Some(header) = accept_encoding {
            let encoding_type = HttpEncoding::parse_accept_encoding(header);
            if !encoding_type.is_empty() {
                match encoding_type[0].0 {
                    HttpEncoding::Gzip => "gzip",
                    HttpEncoding::Deflate => "deflate",
                    HttpEncoding::Brotli => "br",
                }
            } else {
                "identity" // no acceptable encoding found
            }
        } else {
            "identity" // no Accept-Encoding header, no compression
        };

        let body = match response.body() {
            HttpBody::Text(text) => text.into_bytes(),
            HttpBody::Binary(bin) => bin,
        };

        let compressed_body = match encoding {
            "gzip" => {
                let mut encoder = libflate::gzip::Encoder::new(Vec::new()).unwrap();
                std::io::copy(&mut &body[..], &mut encoder).unwrap();
                encoder.finish().into_result().unwrap()
            }
            "deflate" => {
                let mut encoder = libflate::deflate::Encoder::new(Vec::new());
                std::io::copy(&mut &body[..], &mut encoder).unwrap();
                encoder.finish().into_result().unwrap()
            }
            "br" | "brotli" => {
                let mut encoder = brotli::CompressorWriter::new(Vec::new(), 4096, 5, 22);
                std::io::copy(&mut &body[..], &mut encoder).unwrap();
                encoder.into_inner()
            }
            _ => body, // identity, no compression
        };

        CompressedResponse {
            original: response,
            encoding: encoding.to_string(),
            compressed_body,
        }
    }
}

pub struct CompressedResponse<T: HttpWritable> {
    original: T,
    encoding: String,
    compressed_body: Vec<u8>,
}

impl<T: HttpWritable> HttpWritable for CompressedResponse<T> {
    // Returns original status line
    fn status_line(&self) -> &ResponseStatusLine {
        self.original.status_line()
    }

    // Returns modified headers with Content-Encoding and updated Content-Length
    fn headers(&self) -> HashMap<String, String> {
        let mut headers = self.original.headers().clone();
        headers.remove("Content-Length");

        if self.encoding != "identity" {
            headers.insert("Content-Encoding".to_string(), self.encoding.clone());
        }
        headers.insert(
            "Content-Length".to_string(),
            self.compressed_body.len().to_string(),
        );

        headers
    }

    // Returns compressed body
    fn body(&self) -> HttpBody {
        HttpBody::Binary(self.compressed_body.clone())
    }
}

/// Represents a single route
pub struct Route {
    method: HttpMethod,
    path: String, // /echo/{text}
    handler: fn(
        request: &HttpRequest,
        params: &HashMap<String, String>,
        stream: &mut TcpStream,
        ctx: &server::ServerContext,
    ),
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
        router.get("/files/{filename}", file_handler);
        router.post("/files/{filename}", file_handler);

        router
    }

    /// Registers a POST route
    pub fn post(
        &mut self,
        path: &str,
        handler: fn(
            &HttpRequest,
            &HashMap<String, String>,
            &mut TcpStream,
            ctx: &server::ServerContext,
        ),
    ) {
        let route = Route {
            method: HttpMethod::Post,
            path: path.to_string(),
            handler,
        };

        self.routes.push(route);
    }

    /// Registers a GET route
    pub fn get(
        &mut self,
        path: &str,
        handler: fn(
            &HttpRequest,
            &HashMap<String, String>,
            &mut TcpStream,
            ctx: &server::ServerContext,
        ),
    ) {
        let route = Route {
            method: HttpMethod::Get,
            path: path.to_string(),
            handler,
        };

        self.routes.push(route);
    }

    /// Finds matching route and executes handler
    pub fn route(
        &self,
        request: &HttpRequest,
        stream: &mut TcpStream,
        ctx: &server::ServerContext,
    ) {
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
                        return (route.handler)(request, &params, stream, ctx);
                    }
                }
            }
        }

        let accept_header = request.headers.get("Accept").map(|s| s.as_str());
        let err_response = HttpErrorResponse::new(
            HttpStatusCode::NotFound,
            accept_header,
            "Route not found".to_string(),
        );
        send_response(stream, err_response).unwrap_or_else(|e| {
            HttpWriter::log_writer_error(e, "Router::route - sending 404 response");
        });
    }
}

/// Handler that handles root path
pub fn root_handler(
    request: &HttpRequest,
    _params: &HashMap<String, String>,
    stream: &mut TcpStream,
    _ctx: &server::ServerContext,
) {
    let body = "Welcome to the Rust HTTP Server!".to_string();
    let accept_type = request.headers.get("Accept").map(|s| s.as_str());
    let response = HttpResponse::with_negotiation(HttpStatusCode::Ok, body, accept_type);

    send_response(stream, response).unwrap_or_else(|e| {
        HttpWriter::log_writer_error(e, "root_handler");
    });
}

/// Handler that echoes text parameter
pub fn echo_handler(
    request: &HttpRequest,
    params: &HashMap<String, String>,
    stream: &mut TcpStream,
    _ctx: &server::ServerContext,
) {
    let body = params
        .get("text")
        .map(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    let accept_type = request.headers.get("Accept").map(|s| s.as_str());
    let response = HttpResponse::with_negotiation(HttpStatusCode::Ok, body, accept_type);

    let accept_encoding = request.headers.get("Accept-Encoding").map(|s| s.as_str());
    let compressed_response = CompressionMiddleware::apply(response, accept_encoding);
    send_response(stream, compressed_response).unwrap_or_else(|e| {
        HttpWriter::log_writer_error(e, "echo_handler");
    });
}

/// Handler that returns content of a file
pub fn file_handler(
    request: &HttpRequest,
    params: &HashMap<String, String>,
    stream: &mut TcpStream,
    ctx: &server::ServerContext,
) {
    let filename = params.get("filename").map(|s| s.as_str()).unwrap_or("");
    let file_path = ctx.get_serving_directory().join(filename);

    match request.status_line.method {
        HttpMethod::Get => {
            if let Ok(content) = fs::read_to_string(file_path) {
                let response = HttpResponse::for_file(HttpStatusCode::Ok, filename, content);

                send_response(stream, response).unwrap_or_else(|e| {
                    HttpWriter::log_writer_error(e, "file_handler - sending file content");
                });
            } else {
                let err_response = HttpErrorResponse::for_file(
                    HttpStatusCode::NotFound,
                    filename,
                    format!("File '{}' not found", filename), // Create error message
                );
                send_response(stream, err_response).unwrap_or_else(|e| {
                    HttpWriter::log_writer_error(e, "file_handler - sending 404 response");
                });
            }
        }
        HttpMethod::Post => {
            let content = request.body.as_ref().map_or("", |b| b.as_str());
            match fs::write(&file_path, content) {
                Ok(_) => {
                    let response = HttpResponse::for_file(
                        HttpStatusCode::Created,
                        filename,
                        format!("File '{}' created/updated", filename),
                    );

                    send_response(stream, response).unwrap_or_else(|e| {
                        HttpWriter::log_writer_error(e, "file_handler - sending 200 response");
                    });
                }
                Err(e) => {
                    let err_response = HttpErrorResponse::for_file(
                        HttpStatusCode::InternalServerError,
                        filename,
                        format!("Failed to write file '{}': {}", filename, e),
                    );
                    send_response(stream, err_response).unwrap_or_else(|e| {
                        HttpWriter::log_writer_error(e, "file_handler - sending 500 response");
                    });
                }
            }
        }
        _ => {
            let err_response = HttpErrorResponse::new(
                HttpStatusCode::MethodNotAllowed,
                None,
                "Method not allowed".to_string(),
            );
            send_response(stream, err_response).unwrap_or_else(|e| {
                HttpWriter::log_writer_error(e, "file_handler - sending 405 response");
            });
        }
    }
}

/// Handler that returns User-Agent header
pub fn user_agent_handler(
    request: &HttpRequest,
    _params: &HashMap<String, String>,
    stream: &mut TcpStream,
    _ctx: &server::ServerContext,
) {
    let user_agent = request
        .headers
        .get("User-Agent")
        .map(|s| s.as_str())
        .unwrap_or("Unknown");
    let body = user_agent.to_string();
    let accept_type = request.headers.get("Accept").map(|s| s.as_str());
    let response = HttpResponse::with_negotiation(HttpStatusCode::Ok, body, accept_type);

    send_response(stream, response).unwrap_or_else(|e| {
        HttpWriter::log_writer_error(e, "user_agent_handler");
    });
}
