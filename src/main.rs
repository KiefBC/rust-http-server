use crate::http::server;
use std::{net::TcpListener, thread};

mod http;

/// Entry point for the HTTP server
fn main() {
    let args = parse_command_line();
    let directory = extract_directory(&args);
    let context = server::ServerContext::new(directory.as_deref());

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("\nAccepted Connection: {}", stream.peer_addr().unwrap());
                let ctx = context.clone();
                thread::spawn(move || server::handle_client(stream, ctx));
                println!("Connection Closed, bye!");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn parse_command_line() -> Vec<String> {
    std::env::args().collect()
}

fn extract_directory(args: &[String]) -> Option<String> {
    // Look for --directory flag and get the next argument
    for i in 0..args.len() {
        if args[i] == "--directory" && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
    }
    None
}
