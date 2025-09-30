use std::{
    fs,
    io::Read,
    net::{Shutdown, TcpStream},
    path::{self, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use crate::http::{errors, request, routes, writer};

const RESERVED_NAMES: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

#[derive(Debug, Clone)]
/// Server context holding configuration and state
pub struct ServerContext {
    root_path: PathBuf,
    canon_path: PathBuf,
    request_counter: Arc<AtomicU64>,
}

/// Enum representing access intent for path resolution
#[derive(Debug, Clone, Copy)]
pub enum AccessIntent {
    Read,
    Write,
}

/// Result type for path resolution
pub enum ResolveError {
    Forbidden,
    NotFound,
    Invalid,
    Io,
}

/// Result type for server context initialization
#[derive(Debug)]
pub enum InitError {
    RootUnavailable,
    MissingOrNotDir,
}

/// Result of path resolution
pub struct ResolvedPath {
    path: PathBuf,
    exists: bool,
}

impl ResolvedPath {
    /// Gets the resolved absolute path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Checks if the resolved path exists
    pub fn exists(&self) -> bool {
        self.exists
    }
}

impl ServerContext {
    /// Creates a new ServerContext with an optional directory path
    pub fn new(root_dir: &str) -> Result<Self, InitError> {
        let root_path = PathBuf::from(root_dir);
        let canon_path = fs::canonicalize(&root_path).map_err(|_| InitError::RootUnavailable)?;
        println!("Serving files from: {}", canon_path.display());

        if !canon_path.is_dir() {
            return Err(InitError::MissingOrNotDir);
        }

        let context = ServerContext {
            root_path,
            canon_path,
            request_counter: Arc::new(AtomicU64::new(0)),
        };

        Ok(context)
    }

    /// Returns a monotonically increasing request id for logging
    pub fn next_request_id(&self) -> u64 {
        self.request_counter.fetch_add(1, Ordering::Relaxed)
    }

    /// Resolves a requested path to an absolute path within the serving directory
    pub fn resolve_path(
        &self,
        req_path: &str,
        intent: AccessIntent,
        req_id: u64,
    ) -> Result<ResolvedPath, ResolveError> {
        eprintln!(
            "[request {}][resolve_path] start: intent={:?} raw='{}'",
            req_id, intent, req_path
        );

        let decoded = match percent_decode(req_path) {
            Ok(s) => s,
            Err(_) => {
                eprintln!(
                    "[request {}][resolve_path] invalid: bad percent-encoding",
                    req_id
                );
                return Err(ResolveError::Invalid);
            }
        };
        if decoded.is_empty() {
            eprintln!(
                "[request {}][resolve_path] invalid: empty after decode",
                req_id
            );
            return Err(ResolveError::Invalid);
        }

        if decoded.chars().any(|c| c.is_ascii_control()) {
            eprintln!(
                "[request {}][resolve_path] invalid: contains ASCII control characters",
                req_id
            );
            return Err(ResolveError::Invalid);
        }

        let invalid_win_chars = ['<', '>', ':', '"', '\\', '|', '?', '*'];
        if decoded.chars().any(|c| invalid_win_chars.contains(&c)) {
            eprintln!(
                "[request {}][resolve_path] invalid: contains Windows-invalid characters",
                req_id
            );
            return Err(ResolveError::Invalid);
        }

        let path_obj = PathBuf::from(&decoded);
        if path_obj.components().any(|comp| {
            matches!(
                comp,
                path::Component::RootDir | path::Component::Prefix(_)
            )
        }) {
            eprintln!(
                "[request {}][resolve_path] forbidden: absolute or drive-prefixed path",
                req_id
            );
            return Err(ResolveError::Forbidden);
        }

        if path_obj.components().any(|c| {
            matches!(
                c,
                path::Component::CurDir | path::Component::ParentDir
            )
        }) {
            eprintln!(
                "[request {}][resolve_path] forbidden: contains . or .. segments",
                req_id
            );
            return Err(ResolveError::Forbidden);
        }

        if req_path.contains('\\') {
            eprintln!(
                "[request {}][resolve_path] invalid: raw path contains backslash",
                req_id
            );
            return Err(ResolveError::Invalid);
        }
        if req_path
            .as_bytes()
            .windows(3)
            .any(|w| w == b"%2F" || w == b"%2f" || w == b"%5C" || w == b"%5c")
        {
            eprintln!(
                "[request {}][resolve_path] invalid: percent-encoded path separator",
                req_id
            );
            return Err(ResolveError::Invalid);
        }

        let last_name = path_obj.file_name().ok_or_else(|| {
            eprintln!(
                "[request {}][resolve_path] invalid: no terminal filename component",
                req_id
            );
            ResolveError::Invalid
        })?;
        let last = last_name.to_string_lossy();
        if last.ends_with('.') || last.ends_with(' ') {
            eprintln!(
                "[request {}][resolve_path] invalid: trailing dot or space in filename",
                req_id
            );
            return Err(ResolveError::Invalid);
        }
        let base = last.split('.').next().unwrap_or("").to_ascii_lowercase();
        let is_reserved = RESERVED_NAMES.contains(&base.as_str());
        if is_reserved {
            eprintln!(
                "[request {}][resolve_path] invalid: reserved Windows name '{}'",
                req_id, base
            );
            return Err(ResolveError::Invalid);
        }

        let candidate = self.root_path.join(&decoded);
        eprintln!(
            "[request {}][resolve_path] root={} canon_root={} candidate={}",
            req_id,
            self.root_path.display(),
            self.canon_path.display(),
            candidate.display()
        );

        match intent {
            AccessIntent::Read => {
                // Canonicalize the target itself; must exist for reads
                let canon_candidate = fs::canonicalize(&candidate).map_err(|e| match e.kind() {
                    std::io::ErrorKind::NotFound => ResolveError::NotFound,
                    _ => ResolveError::Io,
                })?;

                if !canon_candidate.starts_with(&self.canon_path) {
                    eprintln!(
                        "[request {}][resolve_path] forbidden: outside root after canonicalize",
                        req_id
                    );
                    return Err(ResolveError::Forbidden);
                }

                Ok(ResolvedPath {
                    path: canon_candidate,
                    exists: true,
                })
            }
            AccessIntent::Write => {
                // Canonicalize the parent; a file may not exist yet
                let parent = candidate.parent().ok_or_else(|| {
                    eprintln!(
                        "[request {}][resolve_path] invalid: missing parent directory",
                        req_id
                    );
                    ResolveError::Invalid
                })?;
                let canon_parent = fs::canonicalize(parent).map_err(|e| match e.kind() {
                    std::io::ErrorKind::NotFound => ResolveError::NotFound,
                    _ => ResolveError::Io,
                })?;
                if !canon_parent.starts_with(&self.canon_path) {
                    eprintln!(
                        "[request {}][resolve_path] forbidden: parent outside root after canonicalize",
                        req_id
                    );
                    return Err(ResolveError::Forbidden);
                }

                let exists = candidate.exists();
                let file_name = last_name.to_os_string();
                Ok(ResolvedPath {
                    path: canon_parent.join(file_name),
                    exists,
                })
            }
        }
    }
}

/// Percent-decodes a path segment. Returns Err on malformed sequences.
fn percent_decode(input: &str) -> Result<String, ()> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                if i + 2 >= bytes.len() {
                    return Err(());
                }
                let high_char = bytes[i + 1] as char;
                let low_char = bytes[i + 2] as char;
                let high_nibble = high_char.to_digit(16).ok_or(())? as u8;
                let low_nibble = low_char.to_digit(16).ok_or(())? as u8;
                let byte = (high_nibble << 4) | low_nibble;
                out.push(byte);
                i += 3;
            }
            ch => {
                out.push(ch);
                i += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| ())
}

/// Handles incoming client connections
pub fn handle_client(mut stream: TcpStream, ctx: ServerContext) {
    loop {
        let req_id = ctx.next_request_id();
        let mut request_bytes: Vec<u8> = Vec::new();
        let mut buffer = [0; 1024];

        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    request_bytes.extend(&buffer[..n]);
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

        // If the peer closed the connection without sending bytes, stop gracefully
        if request_bytes.is_empty() {
            println!("[request {}] peer closed connection (no bytes)", req_id);
            break;
        }

        match request::HttpRequest::parse(&request_bytes) {
            Ok(parse_ok) => {
                eprintln!(
                    "[request {}] {} {}",
                    req_id, parse_ok.status_line.method, parse_ok.status_line.path
                );
                let router = routes::Router::new();
                router.route(&parse_ok, &mut stream, &ctx, req_id);
                if parse_ok
                    .headers
                    .get("Connection")
                    .is_some_and(|v| v.eq_ignore_ascii_case("close"))
                {
                    println!(
                        "[request {}] Connection: close header found, shutting down.",
                        req_id
                    );
                    stream.shutdown(Shutdown::Both).unwrap_or_else(|e| {
                        println!("[request {}] Failed to shutdown: {:?}", req_id, e);
                    });
                    break;
                }
            }
            Err(parse_error) => {
                eprintln!(
                    "[request {}] parse error: {} — sending error response",
                    req_id, parse_error
                );
                let error_response = errors::HttpErrorResponse::new(
                    parse_error.status,
                    parse_error.version,
                    parse_error
                        .headers
                        .get("Connection")
                        .map(|s| s.as_str())
                        .unwrap_or("close"),
                    parse_error.headers.get("Accept").map(|s| s.as_str()),
                    "Parsing failed".to_string(),
                );
                writer::send_response(&mut stream, error_response, req_id).unwrap_or_else(|e| {
                    println!(
                        "[request {}] Failed to send error response: {:?}",
                        req_id, e
                    );
                });
            }
        }
    }
}
