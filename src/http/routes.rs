use std::{collections::HashMap, fmt, fs, io, net::TcpStream, path::Path};

use crate::http::{
    errors::HttpErrorResponse,
    files::{
        mime::mime_type_from_extension,
        reader::read_file_with_range,
        types::{ByteRange, FileReadError, FileReadRequest},
    },
    request::{HttpMethod, HttpRequest},
    response::{
        ContentNegotiable, HttpContentType, HttpResponse, HttpStatusCode, ResponseStatusLine,
    },
    server,
    writer::{send_response, HttpBody, HttpWritable, HttpWriter},
};

/// The minimum body size (in bytes) to consider compression
const MINIMUM_BODY_SIZE: usize = 1024;

/// Represents supported HTTP Encoding types
#[derive(Debug, Clone)]
pub enum HttpEncoding {
    Gzip,
    Deflate,
    Brotli,
    Identity,
}

impl fmt::Display for HttpEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let encoding_str = match self {
            HttpEncoding::Gzip => "gzip",
            HttpEncoding::Deflate => "deflate",
            HttpEncoding::Brotli => "brotli",
            HttpEncoding::Identity => "identity",
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

/// Represents Compression Middleware
pub struct CompressionMiddleware;

impl CompressionMiddleware {
    // Applies compression based on the Accept-Encoding header
    pub fn apply<T: HttpWritable>(
        response: T,
        accept_encoding: Option<&str>,
    ) -> CompressedResponse<T> {
        let body = match response.body() {
            HttpBody::Text(text) => text.into_bytes(),
            HttpBody::Binary(bin) => bin,
        };

        if body.len() < MINIMUM_BODY_SIZE {
            return CompressedResponse {
                original: response,
                encoding: "identity".to_string(),
                compressed_body: body,
            };
        }

        let encoding = accept_encoding.and_then(|header| {
            let types = HttpEncoding::parse_accept_encoding(header);
            types.first().map(|(t, _)| t.clone())
        })
            .unwrap_or(HttpEncoding::Identity);

        let compressed_body = match encoding {
            HttpEncoding::Gzip => Self::compress_gzip(&body),
            HttpEncoding::Deflate => Self::compress_deflate(&body),
            HttpEncoding::Brotli => Self::compress_brotli(&body),
            HttpEncoding::Identity => body,
        };

        CompressedResponse {
            original: response,
            encoding: encoding.to_string(),
            compressed_body,
        }
    }

    fn compress_brotli(body: &[u8]) -> Vec<u8> {
        let mut encoder = brotli::CompressorWriter::new(Vec::new(), 4096, 5, 22);
        io::copy(&mut &body[..], &mut encoder).unwrap();
        encoder.into_inner()
    }

    fn compress_deflate(body: &[u8]) -> Vec<u8> {
        let mut encoder = libflate::deflate::Encoder::new(Vec::new());
        io::copy(&mut &body[..], &mut encoder).unwrap();
        encoder.finish().into_result().unwrap()
    }

    fn compress_gzip(body: &[u8]) -> Vec<u8> {
        let mut encoder = libflate::gzip::Encoder::new(Vec::new()).unwrap();
        io::copy(&mut &body[..], &mut encoder).unwrap();
        encoder.finish().into_result().unwrap()
    }
}

/// Represents a response with applied compression
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
        req_id: u64,
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
        router.get("/chunked/{text}", chunked_handler);

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
            req_id: u64,
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
            req_id: u64,
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
        req_id: u64,
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
                        return (route.handler)(request, &params, stream, ctx, req_id);
                    }
                }
            }
        }

        let accept_header = request.headers.get("Accept").map(|s| s.as_str());

        let err_response = HttpErrorResponse::new(
            HttpStatusCode::NotFound,
            request.status_line.version.clone(),
            request.headers.get("Connection").map_or("", |s| s.as_str()),
            accept_header,
            "Route not found".to_string(),
        );

        send_response(stream, err_response, req_id).unwrap_or_else(|e| {
            HttpWriter::log_writer_error(e, "Router::route - sending 404 response");
        });
    }
}

/// Handler that handles a root path
pub fn root_handler(
    request: &HttpRequest,
    _params: &HashMap<String, String>,
    stream: &mut TcpStream,
    _ctx: &server::ServerContext,
    req_id: u64,
) {
    eprintln!("[request {}][root] handling /", req_id);
    let body = "Welcome to the Rust HTTP Server!".to_string();

    let accept_type = request.headers.get("Accept").map(|s| s.as_str());

    let response = HttpResponse::with_negotiation(
        HttpStatusCode::Ok,
        request.status_line.version.clone(),
        request.headers.get("Connection").map_or("", |s| s.as_str()),
        body,
        accept_type,
        None,
        HttpContentType::PlainText.to_string().as_str(),
    );

    send_response(stream, response, req_id).unwrap_or_else(|e| {
        HttpWriter::log_writer_error(e, "root_handler");
    });
}

/// Basic chunked response handler
pub fn chunked_handler(
    request: &HttpRequest,
    params: &HashMap<String, String>,
    stream: &mut TcpStream,
    _ctx: &server::ServerContext,
    req_id: u64,
) {
    eprintln!("[request {}][chunked] params={:?}", req_id, params);
    let status_line = ResponseStatusLine {
        version: request.status_line.version.clone(),
        status: HttpStatusCode::Ok,
    };

    let body = params
        .get("text")
        .map(|s| s.as_bytes())
        .unwrap_or(b"")
        .to_vec();

    let chunked_headers: HashMap<String, String> = [
        ("Content-Type".to_string(), "text/plain".to_string()),
        ("Transfer-Encoding".to_string(), "chunked".to_string()),
        ("Connection".to_string(), "close".to_string()),
    ]
    .into();

    let response = HttpResponse::new(status_line, chunked_headers, Some(HttpBody::Binary(body)));

    send_response(stream, response, req_id).unwrap_or_else(|e| {
        HttpWriter::log_writer_error(e, "chunked_handler");
    });
}

/// Handler that echoes text parameter
pub fn echo_handler(
    request: &HttpRequest,
    params: &HashMap<String, String>,
    stream: &mut TcpStream,
    _ctx: &server::ServerContext,
    req_id: u64,
) {
    eprintln!("[request {}][echo] params={:?}", req_id, params);
    let body = params
        .get("text")
        .map(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    let accept_type = request.headers.get("Accept").map(|s| s.as_str());

    let response = HttpResponse::with_negotiation(
        HttpStatusCode::Ok,
        request.status_line.version.clone(),
        request.headers.get("Connection").map_or("", |s| s.as_str()),
        body,
        accept_type,
        None,
        HttpContentType::PlainText.to_string().as_str(),
    );

    let accept_encoding = request.headers.get("Accept-Encoding").map(|s| s.as_str());

    let compressed_response = CompressionMiddleware::apply(response, accept_encoding);

    send_response(stream, compressed_response, req_id).unwrap_or_else(|e| {
        HttpWriter::log_writer_error(e, "echo_handler");
    });
}

/// Handler that returns the content of a file
pub fn file_handler(
    request: &HttpRequest,
    params: &HashMap<String, String>,
    stream: &mut TcpStream,
    ctx: &server::ServerContext,
    req_id: u64,
) {
    let filename = params.get("filename").map(|s| s.as_str()).unwrap_or("");
    eprintln!(
        "[request {}][file] method={} raw_path={} filename_param={:?}",
        req_id, request.status_line.method, request.status_line.path, filename
    );

    let conn = request
        .headers
        .get("Connection")
        .map(|s| s.as_str())
        .unwrap_or("");

    match request.status_line.method {
        HttpMethod::Get => {
            match ctx.resolve_path(filename, server::AccessIntent::Read, req_id) {
                Ok(resolved) => {
                    let range_header = request.headers.get("Range");

                    let read_request = if let Some(range_str) = range_header {
                        if let Some(range) = ByteRange::from_header(range_str) {
                            FileReadRequest::Range(resolved.path().to_path_buf(), range)
                        } else {
                            FileReadRequest::Full(resolved.path().to_path_buf())
                        }
                    } else {
                        FileReadRequest::Full(resolved.path().to_path_buf())
                    };

                    let read_result = read_file_with_range(read_request);

                    match read_result {
                        Ok(file_result) => {
                            if let Some((start, end)) = file_result.range {
                                let status_line = ResponseStatusLine {
                                    version: request.status_line.version.clone(),
                                    status: HttpStatusCode::PartialContent,
                                };

                                let mime_type = Path::new(filename)
                                    .extension()
                                    .and_then(|ext| ext.to_str())
                                    .map(mime_type_from_extension)
                                    .unwrap_or("application/octet-stream");

                                let mut headers = HashMap::new();
                                headers.insert("Content-Type".to_string(), mime_type.to_string());
                                headers.insert(
                                    "Content-Length".to_string(),
                                    file_result.body.byte_len().to_string(),
                                );
                                headers.insert(
                                    "Content-Range".to_string(),
                                    format!("bytes {}-{}/{}", start, end, file_result.total_size),
                                );
                                headers.insert("Connection".to_string(), conn.to_string());

                                let response =
                                    HttpResponse::new(status_line, headers, Some(file_result.body));

                                send_response(stream, response, req_id).unwrap_or_else(|e| {
                                    HttpWriter::log_writer_error(
                                        e,
                                        "file_handler - sending range content",
                                    );
                                });
                            } else {
                                let response = HttpResponse::for_file(
                                    HttpStatusCode::Ok,
                                    request.status_line.version.clone(),
                                    conn,
                                    filename,
                                    file_result.body,
                                );

                                send_response(stream, response, req_id).unwrap_or_else(|e| {
                                    HttpWriter::log_writer_error(
                                        e,
                                        "file_handler - sending file content",
                                    );
                                });
                            }
                        }
                        Err(err) => {
                            let status = match err {
                                FileReadError::NotFound(_) => HttpStatusCode::NotFound,
                                FileReadError::IoError(_) => HttpStatusCode::InternalServerError,
                                FileReadError::InvalidRange => HttpStatusCode::BadRequest,
                                _ => HttpStatusCode::InternalServerError,
                            };

                            let err_response = HttpErrorResponse::for_file_error(
                                status,
                                request.status_line.version.clone(),
                                conn,
                                filename,
                                "Reading file content failed".to_string(),
                            );

                            send_response(stream, err_response, req_id).unwrap_or_else(|e| {
                                HttpWriter::log_writer_error(
                                    e,
                                    "file_handler - sending error response",
                                );
                            });
                        }
                    }
                }
                Err(err) => {
                    let status = match err {
                        server::ResolveError::Forbidden => HttpStatusCode::Forbidden,
                        server::ResolveError::NotFound => HttpStatusCode::NotFound,
                        server::ResolveError::Invalid => HttpStatusCode::NotFound,
                        server::ResolveError::Io => HttpStatusCode::InternalServerError,
                    };

                    let err_response = HttpErrorResponse::for_file_error(
                        status,
                        request.status_line.version.clone(),
                        conn,
                        filename,
                        "File resolution failed".to_string(),
                    );

                    send_response(stream, err_response, req_id).unwrap_or_else(|e| {
                        HttpWriter::log_writer_error(
                            e,
                            "file_handler - sending error response (GET)",
                        );
                    });
                }
            }
        }
        HttpMethod::Post => {
            let content = request.body.as_ref().map_or("", |b| b.as_str());

            match ctx.resolve_path(filename, server::AccessIntent::Write, req_id) {
                Ok(resolved) => match fs::write(resolved.path(), content) {
                    Ok(_) => {
                        let status = if resolved.exists() {
                            HttpStatusCode::Ok
                        } else {
                            HttpStatusCode::Created
                        };

                        let response = HttpResponse::for_file_error(
                            status,
                            request.status_line.version.clone(),
                            conn,
                            filename,
                            format!("File '{}' created/updated", filename),
                        );

                        send_response(stream, response, req_id).unwrap_or_else(|e| {
                            HttpWriter::log_writer_error(
                                e,
                                "file_handler - sending success response (POST)",
                            );
                        });
                    }
                    Err(e) => {
                        let err_response = HttpErrorResponse::for_file_error(
                            HttpStatusCode::InternalServerError,
                            request.status_line.version.clone(),
                            conn,
                            filename,
                            format!("Failed to write file '{}': {}", filename, e),
                        );

                        send_response(stream, err_response, req_id).unwrap_or_else(|e| {
                            HttpWriter::log_writer_error(
                                e,
                                "file_handler - sending 500 response (write)",
                            );
                        });
                    }
                },
                Err(err) => {
                    let status = match err {
                        server::ResolveError::Forbidden => HttpStatusCode::Forbidden,
                        server::ResolveError::NotFound => HttpStatusCode::NotFound,
                        server::ResolveError::Invalid => HttpStatusCode::NotFound,
                        server::ResolveError::Io => HttpStatusCode::InternalServerError,
                    };

                    let err_response = HttpErrorResponse::for_file_error(
                        status,
                        request.status_line.version.clone(),
                        conn,
                        filename,
                        "File resolution failed".to_string(),
                    );

                    send_response(stream, err_response, req_id).unwrap_or_else(|e| {
                        HttpWriter::log_writer_error(
                            e,
                            "file_handler - sending error response (POST)",
                        );
                    });
                }
            }
        }
        _ => {
            let err_response = HttpErrorResponse::new(
                HttpStatusCode::MethodNotAllowed,
                request.status_line.version.clone(),
                request.headers.get("Connection").map_or("", |s| s.as_str()),
                None,
                "Method not allowed".to_string(),
            );

            send_response(stream, err_response, req_id).unwrap_or_else(|e| {
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
    req_id: u64,
) {
    eprintln!("[request {}][user-agent]", req_id);
    let user_agent = request
        .headers
        .get("User-Agent")
        .map(|s| s.as_str())
        .unwrap_or("Unknown");

    let body = user_agent.to_string();

    let accept_type = request.headers.get("Accept").map(|s| s.as_str());

    let response = HttpResponse::with_negotiation(
        HttpStatusCode::Ok,
        request.status_line.version.clone(),
        request.headers.get("Connection").map_or("", |s| s.as_str()),
        body,
        accept_type,
        None,
        HttpContentType::PlainText.to_string().as_str(),
    );

    send_response(stream, response, req_id).unwrap_or_else(|e| {
        HttpWriter::log_writer_error(e, "user_agent_handler");
    });
}
