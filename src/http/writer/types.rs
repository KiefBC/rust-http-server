#![allow(dead_code)]
use std::{fmt, io};

// Represents whether to use chunked transfer encoding or not
pub struct ChunkedDecision {
    pub use_chunked: bool,
    pub use_content_length: bool,
    pub warning: Option<String>,
}

/// Represents an HTTP body with a text or binary content
#[derive(Debug, Clone)]
pub enum HttpBody {
    Text(String),
    Binary(Vec<u8>),
}

impl fmt::Display for HttpBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpBody::Text(content) => write!(f, "{}", content),
            HttpBody::Binary(content) => write!(f, "{:?}", content),
        }
    }
}

impl HttpBody {
    /// Returns the byte length of the body
    pub fn byte_len(&self) -> usize {
        match self {
            HttpBody::Text(text) => text.as_bytes().len(),
            HttpBody::Binary(bytes) => bytes.len(),
        }
    }
}

/// Represents the state of the writer
#[derive(Debug, Clone, PartialEq)]
pub(super) enum WriterState {
    Initial,       // Can only write status
    StatusWritten, // Can only write headers
    HeadersOpen,   // Can write/replace headers
    HeadersClosed, // Headers done, can only write body
    BodyWritten,   // Body written, can only complete
    Failed,        // Error occurred, no operations allowed
}

/// Represents the error that can occur during writing
#[derive(Debug)]
pub enum WriterError {
    InvalidState(String),
    IoError(io::Error),
    MissingHeader(String),
    InvalidHeader(String),
    ContentLengthMismatch { declared: usize, actual: usize },
}

impl From<io::Error> for WriterError {
    fn from(error: io::Error) -> Self {
        WriterError::IoError(error)
    }
}
