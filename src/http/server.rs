use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

use crate::http::request::{HttpRequest, HttpVersion};
use crate::http::response::{HttpResponse, HttpStatus};
use crate::http::routes::{echo_handler, Route, Router};

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
            // println!("parsed request:\n{:#?}\n", request);
            let mut router = Router::new();
            router.get("/echo/{text}", echo_handler);

            let headers = HashMap::from([("Content-Length", "0"), ("Connection", "close")]);
            // let response = HttpResponse::new(HttpVersion::Http1_1, HttpStatus::Ok, headers);
            let response = router.route(&request);
            if let Err(e) = stream.write_all(&response.to_bytes()) {
                println!("error writing response: {}", e);
            }
            // println!("response sent:\n{:#?}\n", response);
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
