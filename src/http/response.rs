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
            HttpStatusCode::Ok => write!(f, "200 OK"),
            HttpStatusCode::NotFound => write!(f, "404 Not Found"),
            HttpStatusCode::BadRequest => write!(f, "400 Bad Request"),
            HttpStatusCode::MethodNotAllowed => write!(f, "405 Method Not Allowed"),
        }
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
