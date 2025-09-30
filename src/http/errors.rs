use crate::http::{
    request::{HttpVersion},
    response::{self, ContentNegotiable},
    writer::{HttpBody, HttpWritable},
};
use std::collections::HashMap;

/// Represents an HTTP error response
pub struct HttpErrorResponse {
    pub status_line: response::ResponseStatusLine,
    pub headers: HashMap<String, String>,
    pub body: Option<HttpBody>,
}

impl ContentNegotiable for HttpErrorResponse {
    /// Returns an error response for a file with binary/text content
    fn for_file(
        status: response::HttpStatusCode,
        version: HttpVersion,
        connection_header: &str,
        _filename: &str,
        content: HttpBody,
    ) -> HttpErrorResponse {
        let content_text = match content {
            HttpBody::Text(text) => text,
            HttpBody::Binary(bin) => String::from_utf8_lossy(&bin).to_string(),
        };

        HttpErrorResponse::new(
            status,
            version,
            connection_header,
            None,
            content_text,
        )
    }

    /// Returns an error response for a file operation with an error message
    fn for_file_error(
        status: response::HttpStatusCode,
        version: HttpVersion,
        connection_header: &str,
        _filename: &str,
        content: String,
    ) -> HttpErrorResponse {
        HttpErrorResponse::new(
            status,
            version,
            connection_header,
            None,
            content,
        )
    }

    /// Returns an error response with content negotiation based on Accept header
    fn with_negotiation(
        status_code: response::HttpStatusCode,
        version: HttpVersion,
        connection_header: &str,
        content: String,
        accept_header: Option<&str>,
        _chunked: Option<bool>,
        _mime_type: &str,
    ) -> HttpErrorResponse {
        HttpErrorResponse::new(
            status_code,
            version,
            connection_header,
            accept_header,
            content,
        )
    }
}

impl HttpWritable for HttpErrorResponse {
    /// Returns the status line of the error response
    fn status_line(&self) -> &response::ResponseStatusLine {
        &self.status_line
    }

    /// Returns the headers of the error response
    fn headers(&self) -> HashMap<String, String> {
        self.headers.clone()
    }

    /// Returns the body of the error response
    fn body(&self) -> HttpBody {
        self.body.clone().unwrap_or(HttpBody::Text(String::new()))
    }
}

impl HttpErrorResponse {
    /// Creates a new HttpErrorResponse based on the status code, accept header, and message
    pub fn new(
        status_code: response::HttpStatusCode,
        version: HttpVersion,
        _connection_header: &str,
        accept_header: Option<&str>,
        message: String,
    ) -> HttpErrorResponse {
        let status_line = response::ResponseStatusLine {
            version,
            status: status_code.clone(),
        };

        let accepted_type = match accept_header {
            Some(header_value) => response::HttpContentType::from_accept_header(header_value),
            None => response::HttpContentType::PlainText,
        };

        let body_text = match accepted_type {
            response::HttpContentType::Html => {
                format!("<h1>{}</h1><p>{}</p>", status_code, message)
            }
            response::HttpContentType::Json => format!(
                r#"{{"error": "{}", "code": {}}}"#,
                message, status_code as u16
            ),
            response::HttpContentType::PlainText => message,
            response::HttpContentType::OctetStream => String::new(),
        };

        let body = if body_text.is_empty() {
            None
        } else {
            Some(HttpBody::Text(body_text))
        };

        let headers = HashMap::from([
            ("Content-Type".to_string(), accepted_type.to_string()),
            (
                "content-length".to_string(),
                body.as_ref()
                    .map_or("0".to_string(), |b| match b {
                        HttpBody::Text(t) => t.len().to_string(),
                        HttpBody::Binary(bin) => bin.len().to_string(),
                    }),
            ),
            ("Connection".to_string(), "close".to_string()),
        ]);

        HttpErrorResponse {
            status_line,
            headers,
            body,
        }
    }
}
