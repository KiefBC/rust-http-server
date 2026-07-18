use std::collections::HashMap;
use std::fmt;

use super::types::{HttpBody, HttpContentType, HttpStatusCode, ResponseStatusLine};
use crate::http::request::HttpVersion;
use crate::http::writer::HttpWritable;

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
    fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    /// Returns the body of the response
    fn body(&self) -> Option<&HttpBody> {
        self.body.as_ref()
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

    /// Creates a negotiated error response.
    pub fn error(
        status: HttpStatusCode,
        version: HttpVersion,
        connection: &str,
        accept: Option<&str>,
        message: String,
    ) -> Self {
        let content_type = accept.map_or(HttpContentType::PlainText, |value| {
            HttpContentType::from_accept_header(value)
        });

        let body_text = match content_type {
            HttpContentType::Html => format!("<h1>{status}</h1><p>{message}</p>"),
            HttpContentType::Json => {
                format!(r#"{{"error": "{message}", "code": {}}}"#, status as u16)
            }
            HttpContentType::PlainText => message,
            HttpContentType::OctetStream => String::new(),
        };
        let body = (!body_text.is_empty()).then_some(HttpBody::Text(body_text));

        let headers = HashMap::from([
            ("Content-Type".to_string(), content_type.to_string()),
            (
                "Content-Length".to_string(),
                body.as_ref().map_or(0, HttpBody::byte_len).to_string(),
            ),
            ("Connection".to_string(), connection.to_string()),
        ]);

        Self::new(ResponseStatusLine { version, status }, headers, body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn error_response_uses_requested_connection_policy() {
        let response = HttpResponse::error(
            HttpStatusCode::BadRequest,
            HttpVersion::Http1_1,
            "keep-alive",
            None,
            "bad request".to_string(),
        );

        assert_eq!(
            response.headers.get("Connection").map(String::as_str),
            Some("keep-alive")
        );
    }
}
