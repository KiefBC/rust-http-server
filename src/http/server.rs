use std::{
    io::Read,
    net::{Shutdown, TcpStream},
    path,
};

use crate::http::{errors, request, routes, writer};

#[derive(Debug, Clone)]
pub struct ServerContext {
    dir_path: Option<path::PathBuf>,
    default_dir: path::PathBuf,
}

impl ServerContext {
    /// Creates a new ServerContext with optional directory path
    pub fn new(dir_path: Option<&str>) -> Self {
        let default_dir = path::PathBuf::from("/temp_dir/");
        let dir_path_buf = dir_path.map(path::PathBuf::from);

        if let Some(ref dir) = dir_path_buf {
            if !dir.exists() || !dir.is_dir() {
                println!(
                    "Warning: Provided directory '{}' does not exist or is not a directory. Falling back to default 'www' directory.",
                    dir.display()
                );
                return ServerContext {
                    dir_path: None,
                    default_dir,
                };
            }
        } else {
            // Check if default directory exists
            if !default_dir.exists() || !default_dir.is_dir() {
                println!(
                    "Warning: Default directory does not exist. File serving routes may fail."
                );
            }
        }

        ServerContext {
            dir_path: dir_path_buf,
            default_dir,
        }
    }

    /// Gets the directory to serve files from
    pub fn get_serving_directory(&self) -> &path::PathBuf {
        if let Some(ref dir) = self.dir_path {
            dir
        } else {
            &self.default_dir
        }
    }
}

/// Handles incoming client connections
pub fn handle_client(mut stream: TcpStream, ctx: ServerContext) {
    loop {
        let mut request_bytes: Vec<u8> = Vec::new();
        let mut buffer = [0; 1024];

        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    request_bytes.extend_from_slice(&buffer[..n]);
                    // Check if we have a complete HTTP request (ending with \r\n\r\n)
                    if request_bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                Err(e) => {
                    println!("Failed to read from stream: {}", e);
                    return;
                }
            }
        }

        match request::HttpRequest::parse(&request_bytes) {
            Ok(parse_ok) => {
                let router = routes::Router::new();
                router.route(&parse_ok, &mut stream, &ctx);
                if parse_ok
                    .headers
                    .get("Connection")
                    .is_some_and(|v| v.eq_ignore_ascii_case("close"))
                {
                    println!("Connection: close header found, shutting down connection.");
                    stream.shutdown(Shutdown::Both).unwrap_or_else(|e| {
                        println!("Failed to shutdown connection: {:?}", e);
                    });
                    break;
                }
            }
            Err(parse_error) => {
                let error_response = errors::HttpErrorResponse::new(
                    parse_error.status,
                    parse_error.version,
                    parse_error.headers.get("Accept").map(|s| s.as_str()),
                    "Parsing failed".to_string(),
                );
                writer::send_response(&mut stream, error_response).unwrap_or_else(|e| {
                    println!("Failed to send error response: {:?}", e);
                });
            }
        }
    }
}
