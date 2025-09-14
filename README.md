# HTTP/1.1 Server in Rust

A high-performance HTTP server implementation with compression, content negotiation, and file operations.

Project based on: [CodeCrafters HTTP Protocol Server](https://app.codecrafters.io/courses/http-server/overview)

## Features

- Concurrent client connections
- Persistent HTTP connections (keep-alive)
- HTTP/1.0 and HTTP/1.1 version support
- HTTP compression (gzip, deflate, brotli) with quality-based negotiation
- Content negotiation (JSON, HTML, plain text)
- File serving with read/write operations
- Dynamic routing with path parameters
- Binary-safe data pipeline

## Quick Start

```bash
# Run server on port 4221
cargo run

# Run with custom file directory
cargo run -- --directory /path/to/files
```

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | / | Server welcome message |
| GET | /echo/{text} | Echo service |
| GET | /user-agent | Returns User-Agent header |
| GET | /files/{filename} | Read file |
| POST | /files/{filename} | Write file |

## Example Usage

```bash
# Basic request
curl http://localhost:4221/

# Echo with compression
curl -H "Accept-Encoding: gzip" http://localhost:4221/echo/hello

# Content negotiation
curl -H "Accept: application/json" http://localhost:4221/echo/test

# File operations
curl http://localhost:4221/files/test.txt
curl -X POST -d "content" http://localhost:4221/files/new.txt

# Persistent connections (multiple requests on same connection)
curl --http1.1 http://localhost:4221/echo/first --next http://localhost:4221/echo/second

# Force connection close
curl --http1.1 -H "Connection: close" http://localhost:4221/
```

## Architecture

Built with a modular design featuring state-machine-based response handling, middleware support for compression, and comprehensive error handling throughout the request/response pipeline.