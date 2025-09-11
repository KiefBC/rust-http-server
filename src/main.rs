use std::net::TcpListener;

mod http;

/// Entry point for the HTTP server
fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!(
                    "\naccepted new connection from {}\n",
                    stream.peer_addr().unwrap()
                );
                http::server::handle_client(stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
