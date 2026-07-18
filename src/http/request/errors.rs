use std::collections::HashMap;
use std::fmt;

use super::types::HttpVersion;
use crate::http::response::HttpStatusCode;

/// Represents an error that occurred while parsing an HTTP request
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub status: HttpStatusCode,
    pub version: HttpVersion,
    pub headers: HashMap<String, String>,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ParseError: {}", self.status)
    }
}
