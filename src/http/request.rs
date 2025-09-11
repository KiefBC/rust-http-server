use std::collections::HashMap;

use crate::http::response::HttpStatus;

#[derive(Debug, Clone, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Delete => write!(f, "DELETE"),
        }
    }
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
    pub path: String,
    pub version: HttpVersion,
    pub headers: HashMap<String, String>, // "Content-Type" -> "application/json"
    pub body: Option<String>,
}

impl std::fmt::Display for HttpRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}\r\n", self.method, self.path, self.version)?;
        for (key, value) in &self.headers {
            write!(f, "{}: {}\r\n", key, value)?;
        }
        write!(f, "\r\n")?;
        if let Some(body) = &self.body {
            write!(f, "{}", body)?;
        }
        Ok(())
    }
}

impl HttpRequest {
    pub fn parse(request: Vec<String>) -> Result<Self, HttpStatus> {
        if request.is_empty() {
            return Err(HttpStatus::BadRequest);
        }

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

        let path = request_line[1].to_string();
        if path.trim().is_empty() {
            return Err(HttpStatus::BadRequest);
        }
        println!("{}", path);

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

        let request = HttpRequest {
            method,
            path,
            version,
            headers,
            body: None,
        };

        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Testing: GET /index.html HTTP/1.1\r\nHost: localhost:4221\r\nUser-Agent: curl/7.64.1\r\nAccept: */*\r\n\r\n
    fn test_parse_valid_request() {
        let request_lines = vec![
            "GET / HTTP/1.1".to_string(),
            "Host: localhost".to_string(),
            "User-Agent: curl/7.64.1".to_string(),
            "Accept: */*".to_string(),
            "".to_string(),
        ];

        let request = HttpRequest::parse(request_lines).unwrap();

        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.path, "/");
        assert_eq!(request.version, HttpVersion::Http1_1);
        assert_eq!(request.headers.get("Host").unwrap(), "localhost");
        assert_eq!(request.headers.get("User-Agent").unwrap(), "curl/7.64.1");
        assert_eq!(request.headers.get("Accept").unwrap(), "*/*");
        assert!(request.body.is_none());
    }

    #[test]
    fn test_parse_invalid_method() {
        let request_lines = vec![
            "FETCH / HTTP/1.1".to_string(),
            "Host: localhost".to_string(),
            "".to_string(),
        ];

        let result = HttpRequest::parse(request_lines);
        assert_eq!(result.unwrap_err(), HttpStatus::MethodNotAllowed);
    }

    #[test]
    fn test_parse_invalid_version() {
        let request_lines = vec![
            "GET / HTTP/2.0".to_string(),
            "Host: localhost".to_string(),
            "".to_string(),
        ];

        let result = HttpRequest::parse(request_lines);
        assert_eq!(result.unwrap_err(), HttpStatus::BadRequest);
    }

    #[test]
    fn test_parse_invalid_target() {
        let request_lines = vec![
            "GET /noexist HTTP/1.1".to_string(),
            "Host: localhost".to_string(),
            "".to_string(),
        ];

        let result = HttpRequest::parse(request_lines);
        assert_eq!(result.unwrap_err(), HttpStatus::NotFound);
    }

    #[test]
    fn test_parse_malformed_request_line() {
        let request_lines = vec![
            "GET /".to_string(),
            "Host: localhost".to_string(),
            "".to_string(),
        ];

        let result = HttpRequest::parse(request_lines);
        assert_eq!(result.unwrap_err(), HttpStatus::BadRequest);
    }

    #[test]
    fn test_parse_empty_request() {
        let request_lines: Vec<String> = vec![];

        let result = HttpRequest::parse(request_lines);
        assert_eq!(result.unwrap_err(), HttpStatus::BadRequest);
    }

    #[test]
    fn test_parse_request_with_no_headers() {
        let request_lines = vec!["GET / HTTP/1.1".to_string(), "".to_string()];

        let request = HttpRequest::parse(request_lines).unwrap();

        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.path, "/");
        assert_eq!(request.version, HttpVersion::Http1_1);
        assert!(request.headers.is_empty());
        assert!(request.body.is_none());
    }
}
