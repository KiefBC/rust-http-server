use std::collections::HashMap;

use crate::http::response::{HttpBody, ResponseStatusLine};

/// Writable HTTP entity trait
pub trait HttpWritable {
    fn status_line(&self) -> &ResponseStatusLine;
    fn headers(&self) -> &HashMap<String, String>;
    fn body(&self) -> Option<&HttpBody>;
}
