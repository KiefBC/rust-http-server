use crate::http::{request, response, writer::HttpWritable};
use std::collections::HashMap;

/// Represents an HTTP error response
pub struct HttpErrorResponse {
    pub status_line: response::StatusLine,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    // TODO: Potentially trailers
}

impl HttpWritable for HttpErrorResponse {
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

impl HttpErrorResponse {
    /// Creates a new HttpErrorResponse based on the status code, accept header, and message
    pub fn new(
        status_code: response::HttpStatusCode,
        accept_header: Option<&str>,
        message: String,
    ) -> HttpErrorResponse {
        let status_line = response::StatusLine {
            version: request::HttpVersion::Http1_1,
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
