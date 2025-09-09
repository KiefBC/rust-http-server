use std::collections::HashMap;
use std::net::TcpListener;
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
};

#[derive(Debug, Clone, PartialEq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HttpVersion {
    Http1_1,
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub target: String,
    pub version: HttpVersion,
    pub headers: HashMap<String, String>,
}

impl HttpRequest {
    pub fn parse(request: Vec<String>) -> Result<Self, &'static str> {
        let request_line: Vec<&str> = request[0].split_whitespace().collect();
        if request_line.len() != 3 {
            return Err("Invalid request line");
        }

        let method = match request_line[0] {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "DELETE" => HttpMethod::DELETE,
            _ => return Err("Unsupported HTTP method"),
        };

        let target = request_line[1].to_string();
        if target.is_empty() {
            return Err("Empty request target");
        }
        // We only accept absolute path without query or fragment
        if !target[1..].is_empty() {
            return Err("Invalid request target");
        }

        let version = match request_line[2] {
            "HTTP/1.1" => HttpVersion::Http1_1,
            _ => return Err("Unsupported HTTP version"),
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
        })
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!(
                    "\naccepted new connection from {}",
                    stream.peer_addr().unwrap()
                );
                handle_client(stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_client(mut stream: TcpStream) {
    let mut request_lines: Vec<String> = Vec::new();

    {
        let reader = BufReader::new(&stream);
        for line_result in reader.lines() {
            match line_result {
                Ok(line) => {
                    if line.is_empty() {
                        break;
                    }
                    // println!("read line: {}", line);
                    request_lines.push(line);
                }
                Err(e) => {
                    println!("error reading line: {}", e);
                    break;
                }
            }
        }
    }

    if let Ok(request) = HttpRequest::parse(request_lines) {
        println!("parsed request: {:?}", request);
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        if let Err(e) = stream.write_all(response.as_bytes()) {
            println!("error writing response: {}", e);
        }
    } else {
        println!("failed to parse request");
        let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        if let Err(e) = stream.write_all(response.as_bytes()) {
            println!("error writing response: {}", e);
        }
    }
}
