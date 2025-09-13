use std::collections::HashMap;
use std::io::Write;
use std::net::TcpStream;
use titlecase::Titlecase;

use crate::http::{request::HttpVersion, response::HttpStatusCode};

#[derive(Debug, Clone, PartialEq)]
enum WriterState {
    Initial,       // Can only write status
    StatusWritten, // Can only write headers
    HeadersOpen,   // Can write/replace headers
    HeadersClosed, // Headers done, can only write body
    BodyWritten,   // Body written, can only complete
    Failed,        // Error occurred, no operations allowed
}

#[derive(Debug)]
pub enum WriterError {
    InvalidState(String),
    IoError(std::io::Error),
    MissingHeader(String),
    InvalidHeader(String),
    ContentLengthMismatch { declared: usize, actual: usize },
}

impl From<std::io::Error> for WriterError {
    fn from(error: std::io::Error) -> Self {
        WriterError::IoError(error)
    }
}

/// Represents an HTTP response writer
pub struct HttpWriter<'a> {
    stream: &'a mut TcpStream,
    state: WriterState,
    status_line: Option<String>,
    headers: HashMap<String, String>,
    body: Option<String>,
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
    pub fn write_body(&mut self, body: String) -> Result<(), WriterError> {
        if self.state != WriterState::HeadersClosed {
            self.state = WriterState::Failed;
            return Err(WriterError::InvalidState(
                "Can only write body in HeadersClosed state".to_string(),
            ));
        }

        self.body = Some(body);
        self.state = WriterState::BodyWritten;
        Ok(())
    }

    /// Completes the HTTP response write, ensuring all parts are valid and written
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

        // check header for content-length and ensure it matches body length
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
            self.stream.write_all(
                self.headers
                    .iter()
                    .map(|(k, v)| format!("{}: {}\r\n", k, v))
                    .collect::<String>()
                    .as_bytes(),
            )?;
            self.stream.write_all(b"\r\n")?;
            if self.body.is_some() {
                // if body is present, write it
                self.stream
                    .write_all(self.body.as_ref().unwrap().as_bytes())?;
            }
            self.stream.flush()?;

            Ok(())
        } else {
            Err(WriterError::MissingHeader(
                "Content-Length header is required".to_string(),
            ))
        }
    }

    /// write convenience methods for common responses
    pub fn ok_response(stream: &mut TcpStream, body: String) -> Result<(), WriterError> {
        let mut writer = HttpWriter::new(stream);

        writer.write_status_line(HttpVersion::Http1_1, HttpStatusCode::Ok)?;
        writer.write_header("Content-Length".to_string(), body.len().to_string())?;
        writer.write_header("Content-Type".to_string(), "text/plain".to_string())?;
        writer.write_header("Connection".to_string(), "close".to_string())?;
        writer.finish_headers()?;
        writer.write_body(body)?;
        writer.complete_write()?;

        Ok(())
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

    /// write convenience method for error responses
    pub fn error_response(
        stream: &mut TcpStream,
        status: HttpStatusCode,
        body: String,
    ) -> Result<(), WriterError> {
        let mut writer = HttpWriter::new(stream);

        writer.write_status_line(HttpVersion::Http1_1, status)?;
        writer.write_header("Content-Length".to_string(), body.len().to_string())?;
        writer.write_header("Content-Type".to_string(), "text/plain".to_string())?;
        writer.write_header("Connection".to_string(), "close".to_string())?;
        writer.finish_headers()?;
        writer.write_body(body)?;
        writer.complete_write()?;

        Ok(())
    }
}
