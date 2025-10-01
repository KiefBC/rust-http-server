use crate::http::server;
use std::{env, fs::create_dir_all, net::TcpListener, process};
use threadpool::ThreadPool;

mod http;

const DEFAULT_DIR: &str = "./www";

/// Entry point for the HTTP server
fn main() {
    let args = parse_command_line();
    let flag_dir = extract_directory(&args);
    let root_dir = flag_dir.clone().unwrap_or_else(|| DEFAULT_DIR.to_string());
    if flag_dir.is_none() {
        println!(
            "No directory specified. Using default directory: {}",
            DEFAULT_DIR
        );
    } else {
        println!("Using specified directory: {}", root_dir);
    }

    if let Err(e) = create_dir_all(&root_dir) {
        eprintln!("Failed to create directory {}: {:?}", root_dir, e);
        process::exit(1);
    }

    let context = match server::ServerContext::new(&root_dir) {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Failed to initialize server context: {:?}", e);
            process::exit(1);
        }
    };

    let pool = ThreadPool::new(100);

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                match stream.peer_addr() {
                    Ok(addr) => println!("\nAccepted Connection: {}", addr),
                    Err(_) => println!("\nAccepted Connection: unknown"),
                }
                let ctx = context.clone();
                pool.execute(move || {
                    match server::handle_client(stream, ctx) {
                        Ok(()) => {
                            println!("Connection closed");
                        }
                        Err(status_code) => {
                            println!("Connection closed with status code {}", status_code);
                        }
                    }
                });
            }

            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    pool.join();
}

/// Parses command line arguments into a vector of strings
fn parse_command_line() -> Vec<String> {
    env::args().collect()
}

/// Extracts the directory path from command line arguments
fn extract_directory(args: &[String]) -> Option<String> {
    for i in 0..args.len() {
        if args[i] == "--directory" && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
    }
    None
}
