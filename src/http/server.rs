use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

use crate::http::request::{HttpRequest, HttpVersion};
use crate::http::response::HttpResponse;
use crate::http::routes::Router;

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
            let response = router.route(&request);
            if let Err(e) = stream.write_all(&response.to_bytes()) {
                println!("error writing response: {}", e);
            }
        }
        Err(status) => {
            println!("error parsing request: {:#?}", status);

            let response = HttpResponse::new(
                HttpVersion::Http1_1,
                status,
                HashMap::from([("Content-Length", "0"), ("Connection", "close")]),
            );
            if let Err(e) = stream.write_all(&response.to_bytes()) {
                println!("error writing response: {}", e);
            }
        }
    }
}
