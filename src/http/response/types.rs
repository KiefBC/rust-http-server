#![allow(dead_code)]
use std::fmt;

use crate::http::request::HttpVersion;

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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    PartialContent = 206,
    BadRequest = 400,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    InternalServerError = 500,
    NotImplemented = 501,
}

/// Formats HttpStatus for display
impl fmt::Display for HttpStatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpStatusCode::Ok => write!(f, "200 OK"),
            HttpStatusCode::NotFound => write!(f, "404 Not Found"),
            HttpStatusCode::BadRequest => write!(f, "400 Bad Request"),
            HttpStatusCode::MethodNotAllowed => write!(f, "405 Method Not Allowed"),
            HttpStatusCode::Created => write!(f, "201 Created"),
            HttpStatusCode::NoContent => write!(f, "204 No Content"),
            HttpStatusCode::PartialContent => write!(f, "206 Partial Content"),
            HttpStatusCode::InternalServerError => write!(f, "500 Internal Server Error"),
            HttpStatusCode::Forbidden => write!(f, "403 Forbidden"),
            HttpStatusCode::NotImplemented => write!(f, "501 Not Implemented"),
        }
    }
}

/// Status line of an HTTP response
#[derive(Debug, Clone)]
pub struct ResponseStatusLine {
    pub version: HttpVersion,
    pub status: HttpStatusCode,
}
