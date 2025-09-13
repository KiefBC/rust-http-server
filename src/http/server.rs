use std::io::{BufRead, BufReader};
use std::net::TcpStream;

use crate::http::errors;
use crate::http::request::HttpRequest;
use crate::http::routes::Router;
use crate::http::writer::send_response;

/// Handles incoming client connections
pub fn handle_client(mut stream: TcpStream) {
    let mut request_lines: Vec<String> = Vec::new();

    let reader = BufReader::new(&stream);
    for line_result in reader.lines() {
        match line_result {
            Ok(line) => {
                if line.is_empty() {
                    break;
                }
                request_lines.push(line);
            }
            Err(e) => {
                println!("error reading line: {}", e);
                break;
            }
        }
    }

    match HttpRequest::parse(request_lines) {
        Ok(request) => {
            let router = Router::new();
            router.route(&request, &mut stream);
        }
        Err(parse_error) => {
            let error_response = errors::HttpErrorResponse::new(
                parse_error.status,
                parse_error.headers.get("Accept").map(|s| s.as_str()),
                "Parsing failed".to_string(),
            );
            send_response(&mut stream, error_response).unwrap_or_else(|e| {
                println!("Failed to send error response: {:?}", e);
            });
        }
    }
}
