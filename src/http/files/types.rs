use std::{fmt, io, path};

/// Represents a byte range for partial file reads
#[derive(Debug, Clone)]
pub struct ByteRange {
    pub start: u64,
    pub end: Option<u64>, // None means "to end of file"
}

impl ByteRange {
    /// Parses a Range header value like "bytes=0-999" or "bytes=1000-"
    pub fn from_header(range_header: &str) -> Option<ByteRange> {
        if let Some(range) = range_header.strip_prefix("bytes=") {
            if let Some((start, end)) = range.split_once('-') {
                if let Ok(start) = start.parse::<u64>() {
                    if let Ok(end) = end.parse::<u64>() {
                        return Some(ByteRange {
                            start,
                            end: Some(end),
                        });
                    } else if end.is_empty() {
                        return Some(ByteRange { start, end: None });
                    }
                }
            }
        }

        None
    }
}

/// Represents a request to read a file.
#[derive(Debug, Clone)]
pub enum FileReadRequest {
    Full(path::PathBuf),
    Range(path::PathBuf, ByteRange),
}

/// Represents the result of a file read with metadata
pub struct FileReadResult {
    pub body: crate::http::response::HttpBody,
    pub total_size: u64,
    pub range: Option<(u64, u64)>, // (start, end) if this was a range request
}

/// Represents an error that can occur when reading a file.
#[derive(Debug)]
pub enum FileReadError {
    NotFound(io::Error), // Missing files
    IoError(io::Error),  // Unexpected I/O errors
    InvalidRange,        // Range exceeds file size
}

impl fmt::Display for FileReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(error) => write!(f, "file not found: {error}"),
            Self::IoError(error) => write!(f, "file I/O failed: {error}"),
            Self::InvalidRange => f.write_str("requested byte range is invalid"),
        }
    }
}

impl std::error::Error for FileReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotFound(error) | Self::IoError(error) => Some(error),
            Self::InvalidRange => None,
        }
    }
}
