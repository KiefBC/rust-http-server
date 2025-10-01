use std::collections::HashMap;
use std::io::Write;
use std::net::TcpStream;
use titlecase::Titlecase;

use super::chunked::ChunkedWriter;
use super::traits::HttpWritable;
use super::types::{ChunkedDecision, HttpBody, WriterError, WriterState};
use crate::http::request::HttpVersion;
use crate::http::response::HttpStatusCode;

/// Represents an HTTP response writer
pub struct HttpWriter<'a> {
    stream: &'a mut TcpStream,
    state: WriterState,
    status_line: Option<String>,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
    // TODO: Trailers eventually
}

impl<'a> HttpWriter<'a> {
    /// Creates a new HttpWriter
    pub fn new(stream: &'a mut TcpStream) -> Self {
        HttpWriter {
            stream,
            state: WriterState::Initial,
            status_line: None,
            headers: HashMap::new(),
            body: None,
        }
    }

    /// Writes the status line to the HTTP response
    pub fn write_status_line(
        &mut self,
        version: HttpVersion,
        status: HttpStatusCode,
    ) -> Result<(), WriterError> {
        if self.state != WriterState::Initial {
            self.state = WriterState::Failed;

            return Err(WriterError::InvalidState(
                "Can only write Status Line in Initial state".to_string(),
            ));
        }

        let status_line = format!("{} {}\r\n", version, status);
        self.status_line = Some(status_line);

        self.state = WriterState::StatusWritten;

        Ok(())
    }

    /// Writes a header to the HTTP response
    pub fn write_header(&mut self, a: String, b: String) -> Result<(), WriterError> {
        if self.state != WriterState::StatusWritten && self.state != WriterState::HeadersOpen {
            self.state = WriterState::Failed;
            return Err(WriterError::InvalidState(
                "Can only write headers in StatusWritten or HeadersOpen state".to_string(),
            ));
        }
        self.state = WriterState::HeadersOpen;

        let normalized_key = a.titlecase();

        self.headers.retain(|key, _| !key.eq_ignore_ascii_case(&a));
        self.headers.insert(normalized_key, b);

        Ok(())
    }

    /// Finishes the headers section of the HTTP response, acts as a barrier to writing body
    pub fn finish_headers(&mut self) -> Result<(), WriterError> {
        if self.state != WriterState::HeadersOpen && self.state != WriterState::StatusWritten {
            self.state = WriterState::Failed;
            return Err(WriterError::InvalidState(
                "Can only finish headers in HeadersOpen or StatusWritten state".to_string(),
            ));
        }

        self.state = WriterState::HeadersClosed;

        Ok(())
    }

    /// Writes the body to the HTTP response
    pub fn write_body(&mut self, body: &[u8]) -> Result<(), WriterError> {
        if self.state != WriterState::HeadersClosed {
            self.state = WriterState::Failed;
            return Err(WriterError::InvalidState(
                "Can only write body in HeadersClosed state".to_string(),
            ));
        }

        self.body = Some(body.to_vec());

        self.state = WriterState::BodyWritten;

        Ok(())
    }

    /// Completes the HTTP response writing, ensuring all parts are valid and written
    pub fn complete_write(self) -> Result<(), WriterError> {
        if self.state != WriterState::BodyWritten && self.state != WriterState::HeadersClosed {
            return Err(WriterError::InvalidState(
                "Can only complete in BodyWritten state".to_string(),
            ));
        }

        if self.status_line.is_none() {
            return Err(WriterError::InvalidState(
                "Status line must be written before completing".to_string(),
            ));
        }

        if self.headers.contains_key("Content-Length") {
            let body_len: usize = self.body.as_ref().map_or(0, |b| b.len());
            let content_length = self
                .headers
                .get("Content-Length")
                .unwrap()
                .parse::<usize>()
                .map_err(|_| {
                    WriterError::InvalidHeader("Content-Length must be a valid number".to_string())
                })?;

            if content_length != body_len {
                return Err(WriterError::ContentLengthMismatch {
                    declared: content_length,
                    actual: body_len,
                });
            }

            self.stream
                .write_all(self.status_line.as_ref().unwrap().as_bytes())?;
            for (key, value) in &self.headers {
                self.stream
                    .write_all(format!("{}: {}\r\n", key, value).as_bytes())?;
            }

            self.stream.write_all(b"\r\n")?;
            if self.body.is_some() {
                self.stream
                    .write_all(self.body.as_ref().unwrap().as_slice())?;
            }

            self.stream.flush()?;

            Ok(())
        } else {
            Err(WriterError::MissingHeader(
                "Content-Length header is required".to_string(),
            ))
        }
    }

    /// Logs WriterError with specific context for each error variant
    pub fn log_writer_error(error: WriterError, context: &str) {
        match error {
            WriterError::InvalidState(msg) => {
                eprintln!("[{}] State machine violation: {}", context, msg);
            }
            WriterError::ContentLengthMismatch { declared, actual } => {
                eprintln!("[{}] Content-Length mismatch! Declared: {}, Actual: {} - Response will be malformed!",
                    context, declared, actual);
            }
            WriterError::MissingHeader(header) => {
                eprintln!("[{}] Required header missing: {}", context, header);
            }
            WriterError::IoError(io_err) => {
                eprintln!(
                    "[{}] Network/IO error: {} - Connection may be broken",
                    context, io_err
                );
            }
            WriterError::InvalidHeader(msg) => {
                eprintln!("[{}] Invalid header format: {}", context, msg);
            }
        }
    }
}

/// Sends an HTTP response over the given TcpStream
pub fn send_response<T: HttpWritable>(
    stream: &mut TcpStream,
    response: T,
    req_id: u64,
) -> Result<(), WriterError> {
    let version = response.status_line().version.clone();
    let status = response.status_line().status.clone();
    let headers = response.headers();

    let decision = decide_chunking(&version, &headers);
    if let Some(msg) = &decision.warning {
        eprintln!("[request {}][send_response] {}", req_id, msg);
    }

    if decision.use_chunked {
        let mut effective: HashMap<String, String> = HashMap::new();
        let mut transfer_tokens: Vec<String> = Vec::new();
        for (k, v) in &headers {
            if k.eq_ignore_ascii_case("Content-Length") {
                continue;
            }
            if k.eq_ignore_ascii_case("Transfer-Encoding") {
                transfer_tokens = v
                    .split(',')
                    .map(|token| token.trim())
                    .filter(|token| !token.eq_ignore_ascii_case("chunked") && !token.is_empty())
                    .map(|token| token.to_string())
                    .collect();
                continue;
            }
            effective.insert(k.clone(), v.clone());
        }
        transfer_tokens.push("chunked".to_string());

        effective.insert("Transfer-Encoding".to_string(), transfer_tokens.join(", "));

        let mut writer = ChunkedWriter::new(stream);

        writer.write_status_line(version, status)?;

        for (k, v) in effective {
            writer.write_header(k, v)?;
        }
        writer.finish_headers()?;

        match response.body() {
            HttpBody::Text(text) => writer.write_body(text.as_bytes())?,
            HttpBody::Binary(bytes) => writer.write_body(&bytes)?,
        }

        writer.complete_write()?;

        Ok(())
    } else {
        let mut writer = HttpWriter::new(stream);

        writer.write_status_line(version, status)?;

        for (k, v) in &headers {
            if k.eq_ignore_ascii_case("Transfer-Encoding") {
                continue;
            }
            writer.write_header(k.clone(), v.clone())?;
        }
        writer.finish_headers()?;

        match response.body() {
            HttpBody::Text(text) => writer.write_body(text.as_bytes())?,
            HttpBody::Binary(bytes) => writer.write_body(&bytes)?,
        }

        writer.complete_write()?;

        Ok(())
    }
}

/// Gets a header value by key, case-insensitively
fn get_header_ci<'a>(headers: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
}

/// Checks if a comma-separated header value contains a specific token, case-insensitively
fn contains_token_ci(value: &str, token: &str) -> bool {
    value
        .split(',')
        .map(|s| s.trim())
        .any(|t| t.eq_ignore_ascii_case(token))
}

/// Decides whether to use chunked transfer encoding or Content-Length based on the HTTP version and header
fn decide_chunking(version: &HttpVersion, headers: &HashMap<String, String>) -> ChunkedDecision {
    let te_has_chunked = get_header_ci(headers, "Transfer-Encoding")
        .map(|v| contains_token_ci(v, "chunked"))
        .unwrap_or(false);
    let cl_present = get_header_ci(headers, "Content-Length").is_some();

    match version {
        HttpVersion::Http1_0 => {
            if te_has_chunked {
                ChunkedDecision {
                    use_chunked: false,
                    use_content_length: true,
                    warning: Some(
                        "HTTP/1.0: ignoring Transfer-Encoding: chunked; using Content-Length"
                            .to_string(),
                    ),
                }
            } else {
                ChunkedDecision {
                    use_chunked: false,
                    use_content_length: true,
                    warning: None,
                }
            }
        }
        HttpVersion::Http1_1 => {
            if te_has_chunked {
                ChunkedDecision {
                    use_chunked: true,
                    use_content_length: false,
                    warning: if cl_present {
                        Some("TE present → drop Content-Length".to_string())
                    } else {
                        None
                    },
                }
            } else {
                ChunkedDecision {
                    use_chunked: false,
                    use_content_length: true,
                    warning: None,
                }
            }
        }
    }
}
