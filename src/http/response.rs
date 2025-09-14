#![allow(dead_code)]
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use crate::http::request::HttpVersion;
use crate::http::routes::ContentNegotiable;
use crate::http::writer::HttpWritable;

impl ContentNegotiable for HttpResponse {
    /// Returns a response for a file with appropriate content type
    fn for_file(status: HttpStatusCode, filename: &str, content: String) -> Self {
        let content_type = match Path::new(filename).extension() {
            Some(ext) if ext == "html" => "text/html",
            Some(ext) if ext == "json" => "application/json",
            Some(ext) if ext == "txt" => "text/plain",
            _ => "application/octet-stream", // Default for unknown files
        };

        let status_line = StatusLine {
            version: HttpVersion::Http1_1,
            status: status.clone(),
        };

        let headers = HashMap::from([
            ("Content-Type".to_string(), content_type.to_string()),
            ("Content-Length".to_string(), content.len().to_string()),
            ("Connection".to_string(), "close".to_string()),
        ]);

        let body = Some(content);

        HttpResponse::new(status_line, headers, body)
    }
    /// Determines content type from Accept header and formats body accordingly
    fn with_negotiation(
        status_code: HttpStatusCode,
        content: String,
        accept_header: Option<&str>,
    ) -> Self {
        let accepted_type = match accept_header {
            Some(header_value) => HttpContentType::from_accept_header(header_value),
            None => HttpContentType::PlainText, // default
        };

        let body = match accepted_type {
            HttpContentType::Html => Some(format!("<h1>{}</h1><p>{}</p>", status_code, content)),
            HttpContentType::Json => Some(format!(
                r#"{{"message": "{}", "code": {}}}"#,
                content,
                status_code.clone() as u16
            )),
            HttpContentType::PlainText => Some(content.clone()),
            HttpContentType::OctetStream => None, // No body for octet-stream
        };

        let headers = HashMap::from([
            ("Content-Type".to_string(), accepted_type.to_string()),
            (
                "Content-Length".to_string(),
                body.as_ref()
                    .map_or("0".to_string(), |b| b.len().to_string()),
            ),
            ("Connection".to_string(), "close".to_string()),
        ]);

        let status_line = StatusLine {
            version: HttpVersion::Http1_1,
            status: status_code.clone(),
        };

        HttpResponse::new(status_line, headers, body)
    }
}

/// Represents an HTTP response
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_line: StatusLine,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    // TODO: Trailers eventually
}

impl HttpWritable for HttpResponse {
    fn status_line(&self) -> &StatusLine {
        &self.status_line
    }

    fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    fn body(&self) -> &Option<String> {
        &self.body
    }
}

/// Formats HttpResponse for display
impl fmt::Display for HttpResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}\r\n",
            self.status_line.version, self.status_line.status
        )?;
        for (key, value) in &self.headers {
            write!(f, "{}: {}\r\n", key, value)?;
        }
        write!(f, "\r\n")?;
        if let Some(body) = &self.body {
            write!(f, "{}", body)?;
        }
        Ok(())
    }
}

impl HttpResponse {
    /// Creates a new HttpResponse
    pub fn new(
        status_line: StatusLine,
        headers: HashMap<String, String>,
        body: Option<String>,
    ) -> Self {
        HttpResponse {
            status_line,
            headers,
            body,
        }
    }
}

/// Represents common HTTP content types
pub enum HttpContentType {
    Html,
    Json,
    PlainText,
    OctetStream,
}

impl HttpContentType {
    /// Returns HttpContentType from Accept header string
    pub fn from_accept_header(type_str: &str) -> Self {
        match type_str {
            "text/html" => HttpContentType::Html,
            "application/json" => HttpContentType::Json,
            "text/plain" => HttpContentType::PlainText,
            "application/octet-stream" => HttpContentType::OctetStream,
            _ => HttpContentType::PlainText, // default to plain text
        }
    }
}

impl fmt::Display for HttpContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpContentType::Html => write!(f, "text/html"),
            HttpContentType::Json => write!(f, "application/json"),
            HttpContentType::PlainText => write!(f, "text/plain"),
            HttpContentType::OctetStream => write!(f, "application/octet-stream"),
        }
    }
}

/// HTTP response status codes
#[derive(Debug, Clone, PartialEq)]
pub enum HttpStatusCode {
    Ok = 200,
    Created = 201,
    NoContent = 204,
    NotFound = 404,
    BadRequest = 400,
    MethodNotAllowed = 405,
    InternalServerError = 500,
}

/// Formats HttpStatus for display
impl fmt::Display for HttpStatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpStatusCode::Ok => write!(f, "200 OK"),
            HttpStatusCode::NotFound => write!(f, "404 Not Found"),
            HttpStatusCode::BadRequest => write!(f, "400 Bad Request"),
            HttpStatusCode::MethodNotAllowed => write!(f, "405 Method Not Allowed"),
            HttpStatusCode::Created => write!(f, "201 Created"),
            HttpStatusCode::NoContent => write!(f, "204 No Content"),
            HttpStatusCode::InternalServerError => write!(f, "500 Internal Server Error"),
            _ => write!(f, "520 Unknown Error"), // Fallback
        }
    }
}

/// Status line of an HTTP response
/// // TODO: DRY! Reuse from request.rs instead
// // TODO: version should be using out HttpVersion
#[derive(Debug, Clone)]
pub struct StatusLine {
    pub version: HttpVersion,
    pub status: HttpStatusCode,
}

impl StatusLine {
    /// Returns the path of the status line
    pub fn get_path(&self) -> &HttpStatusCode {
        &self.status
    }

    /// Returns the HTTP version of the status line
    pub fn get_version(&self) -> &HttpVersion {
        &self.version
    }
}
