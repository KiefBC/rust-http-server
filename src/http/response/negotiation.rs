use std::collections::HashMap;
use std::path::Path;

use super::builder::HttpResponse;
use super::types::{HttpContentType, HttpStatusCode, ResponseStatusLine};
use crate::http::files::mime::mime_type_from_extension;
use crate::http::request::HttpVersion;
use crate::http::writer::types::HttpBody;

/// Trait for content negotiation.
pub trait ContentNegotiable {
    /// Negotiates on a per-file basis
    fn for_file(
        status: HttpStatusCode,
        version: HttpVersion,
        connection_header: &str,
        filename: &str,
        content: HttpBody,
    ) -> Self
    where
        Self: Sized;

    /// Negotiates on a per-file basis for errors
    fn for_file_error(
        status: HttpStatusCode,
        version: HttpVersion,
        connection_header: &str,
        filename: &str,
        content: String,
    ) -> Self
    where
        Self: Sized;

    /// Constructs a new HTTP response with content negotiation support.
    fn with_negotiation(
        status_code: HttpStatusCode,
        version: HttpVersion,
        connection_header: &str,
        content: String,
        accept_header: Option<&str>,
        chunked: Option<bool>,
        mime_type: &str,
    ) -> Self;
}

impl ContentNegotiable for HttpResponse {
    fn for_file(
        status: HttpStatusCode,
        version: HttpVersion,
        _connection_header: &str,
        filename: &str,
        content: HttpBody,
    ) -> Self {
        let mime_type = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| mime_type_from_extension(ext))
            .unwrap_or("application/octet-stream");

        let status_line = ResponseStatusLine {
            version,
            status: status.clone(),
        };

        let headers = HashMap::from([
            ("Content-Type".to_string(), mime_type.to_string()),
            ("Content-Length".to_string(), content.byte_len().to_string()),
        ]);

        let body = match content {
            HttpBody::Binary(data) => data,
            HttpBody::Text(text) => text.as_bytes().to_vec(),
        };

        HttpResponse::new(status_line, headers, Some(HttpBody::Binary(body)))
    }

    fn for_file_error(
        status: HttpStatusCode,
        version: HttpVersion,
        _connection_header: &str,
        _filename: &str,
        content: String,
    ) -> Self {
        let content_type = "text/plain";

        let status_line = ResponseStatusLine {
            version,
            status: status.clone(),
        };

        let body = HttpBody::Text(content);

        let headers = HashMap::from([
            ("Content-Type".to_string(), content_type.to_string()),
            ("Content-Length".to_string(), body.byte_len().to_string()),
            ("Connection".to_string(), "close".to_string()),
        ]);

        HttpResponse::new(status_line, headers, Some(body))
    }

    fn with_negotiation(
        status_code: HttpStatusCode,
        version: HttpVersion,
        connection_header: &str,
        content: String,
        accept_header: Option<&str>,
        chunked: Option<bool>,
        _mime_type: &str,
    ) -> Self {
        let accepted_type = match accept_header {
            Some(header_value) => HttpContentType::from_accept_header(header_value),
            None => HttpContentType::PlainText,
        };

        let body = match accepted_type {
            HttpContentType::Html => Some(HttpBody::Text(format!(
                "<h1>{}</h1><p>{}</p>",
                status_code, content
            ))),
            HttpContentType::Json => Some(HttpBody::Text(format!(
                r#"{{"message": "{}", "code": {}}}"#,
                content,
                status_code.clone() as u16
            ))),
            HttpContentType::PlainText => Some(HttpBody::Text(content)),
            HttpContentType::OctetStream => None,
        };

        let mut headers = HashMap::new();

        headers.insert("Content-Type".to_string(), accepted_type.to_string());

        let connection_value = if connection_header.eq_ignore_ascii_case("close") {
            "close"
        } else if version == HttpVersion::Http1_1 {
            "keep-alive"
        } else {
            "close"
        };
        headers.insert("Connection".to_string(), connection_value.to_string());

        if chunked.unwrap_or(false) {
            headers.insert("Transfer-Encoding".to_string(), "chunked".to_string());
        } else {
            headers.insert(
                "Content-Length".to_string(),
                body.as_ref()
                    .map_or("0".to_string(), |b| b.byte_len().to_string()),
            );
        }

        let status_line = ResponseStatusLine {
            version,
            status: status_code,
        };

        HttpResponse::new(status_line, headers, body)
    }
}
