use std::io::{BufRead, BufReader};
use std::net::TcpStream;

use crate::http::request::HttpRequest;
use crate::http::routes::Router;
use crate::http::writer::HttpWriter;

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
        Err(status) => {
            println!("error parsing request: {:#?}", status);

            let body = format!("Error: {}\n", status);

            if let Err(e) = HttpWriter::error_response(&mut stream, status, body) {
                HttpWriter::log_writer_error(e, "server_error_response");
            }
        }
    }
}
