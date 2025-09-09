use std::net::TcpListener;
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!(
                    "accepted new connection from {}",
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
    let mut request_lines = Vec::new();
    let mut read_error = false;

    {
        let reader = BufReader::new(&stream);
        for line_result in reader.lines() {
            match line_result {
                Ok(line) => {
                    if line.is_empty() {
                        break;
                    }
                    request_lines.push(line);
                    println!("read line: {}", request_lines.last().unwrap());
                }
                Err(e) => {
                    println!("error reading line: {}", e);
                    read_error = true;
                    break;
                }
            }
        }
    }

    if read_error {
        let resp = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    if let Some(first) = request_lines.first() {
        println!("Request: {}\n", first);
    }

    let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    if let Err(e) = stream.write_all(response.as_bytes()) {
        println!("error writing response: {}", e);
    }
}
