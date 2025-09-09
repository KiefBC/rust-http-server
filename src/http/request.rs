use std::collections::HashMap;

use crate::http::response::HttpStatus;

#[derive(Debug, Clone, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HttpVersion {
    Http1_1,
}

impl std::fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpVersion::Http1_1 => write!(f, "HTTP/1.1"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub target: String,
    pub version: HttpVersion,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl HttpRequest {
    pub fn parse(request: Vec<String>) -> Result<Self, HttpStatus> {
        let request_line: Vec<&str> = request[0].split_whitespace().collect();
        if request_line.len() != 3 {
            return Err(HttpStatus::BadRequest);
        }

        let method = match request_line[0] {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            _ => return Err(HttpStatus::MethodNotAllowed),
        };

        let target = request_line[1].to_string();
        if target.trim().is_empty() {
            return Err(HttpStatus::BadRequest);
        }
        if target.len() > 1 {
            return Err(HttpStatus::NotFound);
        }

        let version = match request_line[2] {
            "HTTP/1.1" => HttpVersion::Http1_1,
            _ => return Err(HttpStatus::BadRequest),
        };

        let mut headers: HashMap<String, String> = HashMap::new();
        for line in &request[1..] {
            if line.is_empty() {
                break;
            }

            if let Some((key, value)) = line.split_once(": ") {
                headers.insert(key.to_string(), value.to_string());
            }
        }

        Ok(HttpRequest {
            method,
            target,
            version,
            headers,
            body: None,
        })
    }
}
