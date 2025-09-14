use std::collections::HashMap;
use std::fmt;

use crate::http::response::HttpStatusCode;

/// Represents an error that occurred while parsing an HTTP request
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub status: HttpStatusCode,
    pub headers: HashMap<String, String>,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ParseError: {}", self.status)
    }
}

/// Represents HTTP methods
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

/// Represents the status line of an HTTP request
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
    pub fn parse(request: &[u8]) -> Result<Self, ParseError> {
        // we expect at least a request line
        if request.is_empty() {
            return Err(ParseError {
                status: HttpStatusCode::BadRequest,
                headers: HashMap::new(),
            });
        }

        let boundary = Self::find_boundary(request).ok_or(ParseError {
            status: HttpStatusCode::BadRequest,
            headers: HashMap::new(),
        })?;

        let (header_bytes, body_bytes) = request.split_at(boundary);
        let body_bytes = &body_bytes[4..]; // skip the \r\n\r\n

        // parse headers first so we can return them in case of error
        let mut headers: HashMap<String, String> = HashMap::new();
        let header_lines = Self::bytes_to_lines(header_bytes);
        for line in &header_lines[1..] {
            if line.is_empty() {
                continue; // Skip empty lines
            }
            if let Some((key, value)) = line.split_once(':') {
                headers.insert(key.trim().to_string(), value.trim().to_string());
            } else {
                return Err(ParseError {
                    status: HttpStatusCode::BadRequest,
                    headers,
                });
            }
        }

        let mut body = "".to_string();
        if !body_bytes.is_empty() {
            body = Self::bytes_to_lines(body_bytes).join("\n");
        }

        let request_line: Vec<&str> = header_lines[0].split_whitespace().collect();
        if request_line.len() != 3 {
            return Err(ParseError {
                status: HttpStatusCode::BadRequest,
                headers,
            });
        }

        let method = match request_line[0] {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            _ => {
                return Err(ParseError {
                    status: HttpStatusCode::MethodNotAllowed,
                    headers,
                })
            }
        };

        let path = request_line[1].to_string();

        let version = match request_line[2] {
            "HTTP/1.1" => HttpVersion::Http1_1,
            _ => {
                return Err(ParseError {
                    status: HttpStatusCode::BadRequest,
                    headers,
                })
            }
        };

        let status_line = StatusLine {
            method: method.clone(),
            path: path.clone(),
            version: version.clone(),
        };

        let content_length = headers
            .get("Content-Length")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);

        let request = HttpRequest {
            status_line,
            headers,
            body: if content_length > 0 { Some(body) } else { None },
        };

        Ok(request)
    }

    /// Locates the boundary between headers and body in raw HTTP request bytes
    fn find_boundary(bytes: &[u8]) -> Option<usize> {
        bytes.windows(4).position(|window| window == b"\r\n\r\n")
    }

    /// Returns lines from raw bytes
    fn bytes_to_lines(bytes: &[u8]) -> Vec<String> {
        String::from_utf8_lossy(bytes)
            .lines()
            .map(|line| line.to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Testing: GET /index.html HTTP/1.1\r\nHost: localhost:4221\r\nUser-Agent: curl/7.64.1\r\nAccept: */*\r\n\r\n
    fn test_parse_valid_request() {
        let request_bytes =
            b"GET / HTTP/1.1\r\nHost: localhost\r\nUser-Agent: curl/7.64.1\r\nAccept: */*\r\n\r\n";

        let request = HttpRequest::parse(request_bytes).unwrap();

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
        let request_bytes = b"FETCH / HTTP/1.1\r\nHost: localhost\r\n\r\n";

        let result = HttpRequest::parse(request_bytes);
        assert_eq!(
            result.unwrap_err(),
            ParseError {
                status: HttpStatusCode::MethodNotAllowed,
                headers: HashMap::from([("Host".to_string(), "localhost".to_string())]),
            }
        );
    }

    #[test]
    fn test_parse_invalid_version() {
        let request_bytes = b"GET / HTTP/2.0\r\nHost: localhost\r\n\r\n";

        let result = HttpRequest::parse(request_bytes);
        assert_eq!(
            result.unwrap_err(),
            ParseError {
                status: HttpStatusCode::BadRequest,
                headers: HashMap::from([("Host".to_string(), "localhost".to_string())]),
            }
        );
    }

    #[test]
    fn test_parse_invalid_target() {
        let request_bytes = b"GET /noexist HTTP/1.1\r\nHost: localhost\r\n\r\n";

        let result = HttpRequest::parse(request_bytes);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_malformed_request_line() {
        let request_bytes = b"GET /\r\nHost: localhost\r\n\r\n";

        let result = HttpRequest::parse(request_bytes);
        assert_eq!(
            result.unwrap_err(),
            ParseError {
                status: HttpStatusCode::BadRequest,
                headers: HashMap::from([("Host".to_string(), "localhost".to_string())]),
            }
        );
    }

    #[test]
    fn test_parse_empty_request() {
        let request_bytes = b"";

        let result = HttpRequest::parse(request_bytes);
        assert_eq!(
            result.unwrap_err(),
            ParseError {
                status: HttpStatusCode::BadRequest,
                headers: HashMap::new(),
            }
        );
    }

    #[test]
    fn test_parse_request_with_no_headers() {
        let request_bytes = b"GET / HTTP/1.1\r\n\r\n";

        let request = HttpRequest::parse(request_bytes).unwrap();

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
