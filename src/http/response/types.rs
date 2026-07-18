use std::fmt;

use crate::http::request::HttpVersion;

/// A textual or binary HTTP response body.
#[derive(Debug, Clone)]
pub enum HttpBody {
    Text(String),
    Binary(Vec<u8>),
}

impl HttpBody {
    /// Returns the encoded byte length of the body.
    pub fn byte_len(&self) -> usize {
        match self {
            Self::Text(text) => text.len(),
            Self::Binary(bytes) => bytes.len(),
        }
    }

    /// Returns the body as bytes.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Text(text) => text.as_bytes(),
            Self::Binary(bytes) => bytes,
        }
    }
}

impl fmt::Display for HttpBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(content) => f.write_str(content),
            Self::Binary(content) => write!(f, "{content:?}"),
        }
    }
}

/// Represents common HTTP content types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpStatusCode {
    Ok = 200,
    Created = 201,
    PartialContent = 206,
    BadRequest = 400,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    InternalServerError = 500,
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
            HttpStatusCode::PartialContent => write!(f, "206 Partial Content"),
            HttpStatusCode::InternalServerError => write!(f, "500 Internal Server Error"),
            HttpStatusCode::Forbidden => write!(f, "403 Forbidden"),
        }
    }
}

/// Status line of an HTTP response
#[derive(Debug, Clone)]
pub struct ResponseStatusLine {
    pub version: HttpVersion,
    pub status: HttpStatusCode,
}
