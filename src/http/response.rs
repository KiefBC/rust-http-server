use crate::http::request::HttpVersion;
use std::collections::HashMap;
use std::fmt;

/// Represents an HTTP response
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_line: StatusLine,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    // TODO: Trailers eventually
}

/// HTTP response status codes
#[derive(Debug, Clone, PartialEq)]
pub enum HttpStatusCode {
    Ok = 200,
    NotFound = 404,
    InternalServerError = 500,
    BadRequest = 400,
    MethodNotAllowed = 405,
}

/// Status line of an HTTP response
#[derive(Debug, Clone)]
pub struct StatusLine {
    pub version: String,
    pub status: HttpStatusCode,
}

/// Formats HttpStatus for display
impl fmt::Display for HttpStatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpStatusCode::Ok => write!(f, "Ok"),
            HttpStatusCode::NotFound => write!(f, "Not Found"),
            HttpStatusCode::InternalServerError => write!(f, "Internal Server Error"),
            HttpStatusCode::BadRequest => write!(f, "Bad Request"),
            HttpStatusCode::MethodNotAllowed => write!(f, "Method Not Allowed"),
        }
    }
}

/// Formats HttpResponse for display
impl fmt::Display for HttpResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}\r\n",
            self.status_line.version,
            self.status_line.status.format()
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

impl HttpStatusCode {
    /// Returns numeric status code
    pub fn code(&self) -> u16 {
        self.clone() as u16
    }

    /// Returns status text
    pub fn text(&self) -> &str {
        match self {
            HttpStatusCode::Ok => "OK",
            HttpStatusCode::NotFound => "Not Found",
            HttpStatusCode::InternalServerError => "Internal Server Error",
            HttpStatusCode::BadRequest => "Bad Request",
            HttpStatusCode::MethodNotAllowed => "Method Not Allowed",
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
    pub fn new(version: HttpVersion, status: HttpStatusCode, headers: HashMap<&str, &str>) -> Self {
        let version = version.to_string();

        let mut header_map = HashMap::new();
        for (key, value) in headers {
            header_map.insert(key.to_string(), value.to_string());
        }

        let status_line = StatusLine {
            version: version.clone(),
            status: status.clone(),
        };

        HttpResponse {
            status_line,
            headers: header_map,
            body: None,
        }
    }
}
