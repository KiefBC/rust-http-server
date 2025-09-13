use std::collections::HashMap;
use std::fmt;

use crate::http::request::HttpVersion;
use crate::http::response;
use crate::http::writer::HttpWritable;

/// Represents an HTTP response
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_line: StatusLine,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    // TODO: Trailers eventually
}

impl HttpWritable for HttpResponse {
    fn status_line(&self) -> &response::StatusLine {
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

/// Represents common HTTP content types
pub enum HttpContentType {
    Html,
    Json,
    PlainText,
}

impl HttpContentType {
    pub fn from_accept_header(type_str: &str) -> Self {
        match type_str {
            "text/html" => HttpContentType::Html,
            "application/json" => HttpContentType::Json,
            "text/plain" => HttpContentType::PlainText,
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
        }
    }
}

/// HTTP response status codes
#[derive(Debug, Clone, PartialEq)]
pub enum HttpStatusCode {
    Ok = 200,
    NotFound = 404,
    BadRequest = 400,
    MethodNotAllowed = 405,
}

/// Formats HttpStatus for display
impl fmt::Display for HttpStatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpStatusCode::Ok => write!(f, "200 OK"),
            HttpStatusCode::NotFound => write!(f, "404 Not Found"),
            HttpStatusCode::BadRequest => write!(f, "400 Bad Request"),
            HttpStatusCode::MethodNotAllowed => write!(f, "405 Method Not Allowed"),
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
