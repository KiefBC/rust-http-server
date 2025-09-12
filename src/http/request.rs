use std::collections::HashMap;
use std::fmt;

use crate::http::response::HttpStatusCode;

/// HTTP request methods
#[derive(Debug, Clone, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

/// Formats HttpMethod for display
impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Delete => write!(f, "DELETE"),
        }
    }
}

/// HTTP protocol versions
#[derive(Debug, Clone, PartialEq)]
pub enum HttpVersion {
    Http1_1,
}

/// Formats HttpVersion for display
impl fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpVersion::Http1_1 => write!(f, "HTTP/1.1"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatusLine {
    pub method: HttpMethod,
    pub path: String,
    pub version: HttpVersion,
}

/// Represents an HTTP request
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub status_line: StatusLine,
    pub headers: HashMap<String, String>, // "Content-Type" -> "application/json"
    pub body: Option<String>,
    // TODO: Trailers and etc
}

/// Formats HttpRequest for display
impl fmt::Display for HttpRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}\r\n",
            self.status_line.method, self.status_line.path, self.status_line.version
        )?;
        let mut headers: Vec<_> = self.headers.iter().collect();
        headers.sort_by_key(|(key, _)| *key);
        for (key, value) in headers {
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
    /// Parses raw request lines into HttpRequest
    pub fn parse(request: Vec<String>) -> Result<Self, HttpStatusCode> {
        if request.is_empty() {
            return Err(HttpStatusCode::BadRequest);
        }

        let request_line: Vec<&str> = request[0].split_whitespace().collect();
        if request_line.len() != 3 {
            return Err(HttpStatusCode::BadRequest);
        }

        let method = match request_line[0] {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            _ => return Err(HttpStatusCode::MethodNotAllowed),
        };

        let path = request_line[1].to_string();

        let version = match request_line[2] {
            "HTTP/1.1" => HttpVersion::Http1_1,
            _ => return Err(HttpStatusCode::BadRequest),
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

        let status_line = StatusLine {
            method: method.clone(),
            path: path.clone(),
            version: version.clone(),
        };

        let request = HttpRequest {
            status_line,
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

        assert_eq!(request.status_line.method, HttpMethod::Get);
        assert_eq!(request.status_line.path, "/");
        assert_eq!(request.status_line.version, HttpVersion::Http1_1);
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
        assert_eq!(result.unwrap_err(), HttpStatusCode::MethodNotAllowed);
    }

    #[test]
    fn test_parse_invalid_version() {
        let request_lines = vec![
            "GET / HTTP/2.0".to_string(),
            "Host: localhost".to_string(),
            "".to_string(),
        ];

        let result = HttpRequest::parse(request_lines);
        assert_eq!(result.unwrap_err(), HttpStatusCode::BadRequest);
    }

    #[test]
    fn test_parse_invalid_target() {
        let request_lines = vec![
            "GET /noexist HTTP/1.1".to_string(),
            "Host: localhost".to_string(),
            "".to_string(),
        ];

        let result = HttpRequest::parse(request_lines);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_malformed_request_line() {
        let request_lines = vec![
            "GET /".to_string(),
            "Host: localhost".to_string(),
            "".to_string(),
        ];

        let result = HttpRequest::parse(request_lines);
        assert_eq!(result.unwrap_err(), HttpStatusCode::BadRequest);
    }

    #[test]
    fn test_parse_empty_request() {
        let request_lines: Vec<String> = vec![];

        let result = HttpRequest::parse(request_lines);
        assert_eq!(result.unwrap_err(), HttpStatusCode::BadRequest);
    }

    #[test]
    fn test_parse_request_with_no_headers() {
        let request_lines = vec!["GET / HTTP/1.1".to_string(), "".to_string()];

        let request = HttpRequest::parse(request_lines).unwrap();

        assert_eq!(request.status_line.method, HttpMethod::Get);
        assert_eq!(request.status_line.path, "/");
        assert_eq!(request.status_line.version, HttpVersion::Http1_1);
        assert!(request.headers.is_empty());
    }

    #[test]
    fn test_http_method_display() {
        let methods: Vec<HttpMethod> = vec![
            HttpMethod::Get,
            HttpMethod::Post,
            HttpMethod::Put,
            HttpMethod::Delete,
        ];

        let expected = vec!["GET", "POST", "PUT", "DELETE"];

        assert_eq!(
            methods
                .iter()
                .map(|m| m.to_string())
                .collect::<Vec<String>>(),
            expected
        );
    }

    #[test]
    fn test_http_version_display() {
        let version = HttpVersion::Http1_1;
        let expected = "HTTP/1.1";
        assert_eq!(version.to_string(), expected);
    }

    #[test]
    fn test_http_request_display_no_body() {
        let status_line = StatusLine {
            method: HttpMethod::Get,
            path: "/".to_string(),
            version: HttpVersion::Http1_1,
        };

        let request = HttpRequest {
            status_line,
            headers: HashMap::from([
                ("Host".to_string(), "localhost".to_string()),
                ("User-Agent".to_string(), "curl/7.64.1".to_string()),
            ]),
            body: None,
        };

        let expected = "GET / HTTP/1.1\r\nHost: localhost\r\nUser-Agent: curl/7.64.1\r\n\r\n";

        assert_eq!(request.to_string(), expected);
    }

    #[test]
    fn test_http_request_display_with_body() {
        let status_line = StatusLine {
            method: HttpMethod::Get,
            path: "/".to_string(),
            version: HttpVersion::Http1_1,
        };

        let request = HttpRequest {
            status_line,
            headers: HashMap::from([
                ("Host".to_string(), "localhost".to_string()),
                ("User-Agent".to_string(), "curl/7.64.1".to_string()),
            ]),
            body: Some("Hello, World!".to_string()),
        };

        let expected =
            "GET / HTTP/1.1\r\nHost: localhost\r\nUser-Agent: curl/7.64.1\r\n\r\nHello, World!";

        assert_eq!(request.to_string(), expected);
    }
}
