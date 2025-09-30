use std::{collections::HashMap, io::Write, net::TcpStream};

use titlecase::Titlecase;

use super::types::{WriterError, WriterState};
use crate::http::{request::HttpVersion, response::HttpStatusCode};

/// A writer for HTTP responses that uses chunked transfer encoding.
pub struct ChunkedWriter<'a> {
    stream: &'a mut TcpStream,
    state: WriterState,
    status_line: Option<String>,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
}

impl<'a> ChunkedWriter<'a> {
    /// Create a new ChunkedWriter with the given TcpStream
    pub fn new(stream: &'a mut TcpStream) -> Self {
        ChunkedWriter {
            stream,
            state: WriterState::Initial,
            status_line: None,
            headers: HashMap::new(),
            body: None,
        }
    }

    /// Write the status line of the HTTP response. This can only be called once.
    pub fn write_status_line(
        &mut self,
        version: HttpVersion,
        status: HttpStatusCode,
    ) -> Result<(), WriterError> {
        if self.state != WriterState::Initial {
            self.state = WriterState::Failed;
            return Err(WriterError::InvalidState(
                "[request {req_id}][send_response] Cannot write status line in current state"
                    .into(),
            ));
        }

        let status_line = format!("{} {}\r\n", version, status);
        self.status_line = Some(status_line);
        self.state = WriterState::StatusWritten;

        Ok(())
    }

    /// Write or replace a header. This can only be called after the status line is written and before headers are finished.
    pub fn write_header(&mut self, key: String, value: String) -> Result<(), WriterError> {
        if self.state != WriterState::StatusWritten && self.state != WriterState::HeadersOpen {
            self.state = WriterState::Failed;

            return Err(WriterError::InvalidState(
                "[request {req_id}][send_response] Cannot write headers in current state".into(),
            ));
        }

        self.state = WriterState::HeadersOpen;

        let normalized_key = key.titlecase();

        self.headers
            .retain(|existing_key, _| !existing_key.eq_ignore_ascii_case(&key));
        self.headers.insert(normalized_key, value);

        Ok(())
    }

    /// Finish writing headers. This must be called before writing the body.
    pub fn finish_headers(&mut self) -> Result<(), WriterError> {
        if self.state != WriterState::StatusWritten && self.state != WriterState::HeadersOpen {
            self.state = WriterState::Failed;
            return Err(WriterError::InvalidState(
                "[request {req_id}][send_response] Cannot end headers in current state".into(),
            ));
        }

        self.state = WriterState::HeadersClosed;
        Ok(())
    }

    /// Write the body of the response. This can only be called after headers are finished.
    pub fn write_body(&mut self, body: &[u8]) -> Result<(), WriterError> {
        if self.state != WriterState::HeadersClosed {
            self.state = WriterState::Failed;

            return Err(WriterError::InvalidState(
                "[request {req_id}][send_response] Cannot write body in current state".into(),
            ));
        }

        if !body.is_empty() {
            self.body = Some(body.to_vec());
        }

        self.state = WriterState::BodyWritten;

        Ok(())
    }

    /// Complete the writing process by sending the status line, headers, and body in chunked transfer encoding
    pub fn complete_write(self) -> Result<(), WriterError> {
        // Empty body allowed in chunked encoding
        if self.state != WriterState::BodyWritten && self.state != WriterState::HeadersClosed {
            return Err(WriterError::InvalidState(
                "[request {req_id}][send_response] Cannot complete write in current state".into(),
            ));
        }

        let status_line = self.status_line.ok_or_else(|| {
            WriterError::InvalidState(
                "[request {req_id}][send_response] Status line must be set before completing write"
                    .into(),
            )
        })?;

        if self.headers.is_empty() {
            return Err(WriterError::InvalidState(
                "[request {req_id}][send_response] At least one header must be set before completing write"
                    .into(),
            ));
        }

        if self.headers.get("Transfer-Encoding").map(|v| v.as_str()) != Some("chunked") {
            return Err(WriterError::InvalidState(
                "[request {req_id}][send_response] 'Transfer-Encoding: chunked' header must be set before completing write"
                    .into(),
            ));
        }

        if self.headers.contains_key("Content-Length") {
            return Err(WriterError::InvalidState(
                "[request {req_id}][send_response] 'Content-Length' header must not be set when using chunked transfer encoding"
                    .into(),
            ));
        }

        write!(self.stream, "{}", status_line).map_err(WriterError::IoError)?;

        for (key, value) in &self.headers {
            write!(self.stream, "{}: {}\r\n", key, value).map_err(WriterError::IoError)?;
        }
        write!(self.stream, "\r\n").map_err(WriterError::IoError)?;

        let body_opt = self.body.clone();
        if let Some(body) = body_opt {
            Self::write_chunk(self.stream, &body)?;
        }

        write!(self.stream, "0\r\n\r\n").map_err(WriterError::IoError)?;
        self.stream.flush().map_err(WriterError::IoError)?;

        Ok(())
    }

    /// Write a chunk of data in chunked transfer encoding
    fn write_chunk(stream: &mut TcpStream, data: &[u8]) -> Result<(), WriterError> {
        let chunk_size = data.len();
        let chunk_header = format!("{:x}\r\n", chunk_size);
        stream
            .write_all(chunk_header.as_bytes())
            .map_err(WriterError::IoError)?;

        let chunk_data = &data[..chunk_size];
        stream.write_all(chunk_data).map_err(WriterError::IoError)?;
        stream.write_all(b"\r\n").map_err(WriterError::IoError)?;

        Ok(())
    }
}
