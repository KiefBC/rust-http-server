use std::collections::HashMap;
use std::fmt;

use super::types::ResponseStatusLine;
use crate::http::writer::{HttpBody, HttpWritable};

/// Represents an HTTP response
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_line: ResponseStatusLine,
    pub headers: HashMap<String, String>,
    pub body: Option<HttpBody>,
    // TODO: Trailers eventually
}

impl HttpWritable for HttpResponse {
    /// Returns the status line of the response
    fn status_line(&self) -> &ResponseStatusLine {
        &self.status_line
    }

    /// Returns the headers of the response
    fn headers(&self) -> HashMap<String, String> {
        self.headers.clone()
    }

    /// Returns the body of the response
    fn body(&self) -> HttpBody {
        self.body.clone().unwrap_or(HttpBody::Text(String::new()))
    }
}

impl fmt::Display for HttpResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
        status_line: ResponseStatusLine,
        headers: HashMap<String, String>,
        body: Option<HttpBody>,
    ) -> Self {
        HttpResponse {
            status_line,
            headers,
            body,
        }
    }
}
