use std::collections::HashMap;

use crate::http::response::ResponseStatusLine;
use super::types::HttpBody;

/// Writable HTTP entity trait
pub trait HttpWritable {
    fn status_line(&self) -> &ResponseStatusLine;
    fn headers(&self) -> HashMap<String, String>;
    fn body(&self) -> HttpBody;
}