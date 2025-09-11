use crate::http::request::HttpVersion;
use std::collections::HashMap;

/// Represents an HTTP response
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub version: String,
    pub status: HttpStatus,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

/// HTTP response status codes
#[derive(Debug, Clone, PartialEq)]
pub enum HttpStatus {
    Ok = 200,
    NotFound = 404,
    InternalServerError = 500,
    BadRequest = 400,
    MethodNotAllowed = 405,
}

/// Formats HttpStatus for display
impl std::fmt::Display for HttpStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpStatus::Ok => write!(f, "Ok"),
            HttpStatus::NotFound => write!(f, "Not Found"),
            HttpStatus::InternalServerError => write!(f, "Internal Server Error"),
            HttpStatus::BadRequest => write!(f, "Bad Request"),
            HttpStatus::MethodNotAllowed => write!(f, "Method Not Allowed"),
        }
    }
}

/// Formats HttpResponse for display
impl std::fmt::Display for HttpResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}\r\n", self.version, self.status.format())?;
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

impl HttpStatus {
    /// Returns numeric status code
    pub fn code(&self) -> u16 {
        self.clone() as u16
    }

    /// Returns status text
    pub fn text(&self) -> &str {
        match self {
            HttpStatus::Ok => "OK",
            HttpStatus::NotFound => "Not Found",
            HttpStatus::InternalServerError => "Internal Server Error",
            HttpStatus::BadRequest => "Bad Request",
            HttpStatus::MethodNotAllowed => "Method Not Allowed",
        }
    }

    /// Formats status code and text
    pub fn format(&self) -> String {
        format!("{} {}", self.code(), self.text())
    }
}

impl HttpResponse {
    /// Converts response to bytes for transmission
    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_string().into_bytes()
    }

    /// Creates a new HTTP response
    pub fn new(version: HttpVersion, status: HttpStatus, headers: HashMap<&str, &str>) -> Self {
        let version = version.to_string();

        let mut header_map = HashMap::new();
        for (key, value) in headers {
            header_map.insert(key.to_string(), value.to_string());
        }

        HttpResponse {
            version,
            status,
            headers: header_map,
            body: None,
        }
    }
}
