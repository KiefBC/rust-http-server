use crate::http::{
    request::{self, HttpVersion},
    response,
    routes::ContentNegotiable,
    writer::{HttpBody, HttpWritable},
};
use std::collections::HashMap;

/// Represents an HTTP error response
pub struct HttpErrorResponse {
    pub status_line: response::ResponseStatusLine,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    // TODO: Potentially trailers
}

impl ContentNegotiable for HttpErrorResponse {
    /// Returns an error response for a given file with content negotiation
    fn for_file(
        status: response::HttpStatusCode,
        version: request::HttpVersion,
        connection_header: &str,
        _filename: &str,
        content: String,
    ) -> HttpErrorResponse {
        // For simplicity, we ignore filename-based negotiation here
        HttpErrorResponse::new(status, version, connection_header, None, content)
    }

    /// Returns an error response with content negotiation based on Accept header
    fn with_negotiation(
        status_code: response::HttpStatusCode,
        version: request::HttpVersion,
        connection_header: &str,
        content: String,
        accept_header: Option<&str>,
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
        HttpBody::Text(self.body.clone().unwrap_or_default())
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
            None => response::HttpContentType::PlainText, // default
        };

        let body = match accepted_type {
            response::HttpContentType::Html => {
                Some(format!("<h1>{}</h1><p>{}</p>", status_code, message))
            }
            response::HttpContentType::Json => Some(format!(
                r#"{{"error": "{}", "code": {}}}"#,
                message, status_code as u16
            )),
            response::HttpContentType::PlainText => Some(message.clone()),
            response::HttpContentType::OctetStream => None, // No body for octet-stream
        };

        let headers = HashMap::from([
            ("Content-Type".to_string(), accepted_type.to_string()),
            (
                "content-length".to_string(),
                body.as_ref()
                    .map_or("0".to_string(), |b| b.len().to_string()),
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
