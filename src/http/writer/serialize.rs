use std::collections::{BTreeMap, HashMap};
use std::io::{self, Write};

use super::traits::HttpWritable;
use crate::http::request::HttpVersion;
#[cfg(test)]
use crate::http::response::HttpStatusCode;
use crate::http::response::ResponseStatusLine;

/// Sends an HTTP response over the provided output stream.
pub fn send_response<W: Write, T: HttpWritable>(
    output: &mut W,
    response: T,
    _request_id: u64,
) -> io::Result<()> {
    let body = response.body().map_or(&[][..], |body| body.as_bytes());
    write_response(output, response.status_line(), response.headers(), body)
}

fn write_response<W: Write>(
    output: &mut W,
    status_line: &ResponseStatusLine,
    source_headers: &HashMap<String, String>,
    body: &[u8],
) -> io::Result<()> {
    let mut headers = BTreeMap::new();
    for (key, value) in source_headers {
        insert_header(&mut headers, key.clone(), value.clone());
    }

    let transfer_encoding = remove_header(&mut headers, "Transfer-Encoding");
    let use_chunked = status_line.version == HttpVersion::Http1_1
        && transfer_encoding
            .as_deref()
            .is_some_and(|value| contains_token(value, "chunked"));

    if use_chunked {
        remove_header(&mut headers, "Content-Length");

        let mut codings = transfer_encoding
            .as_deref()
            .into_iter()
            .flat_map(|value| value.split(','))
            .map(str::trim)
            .filter(|coding| !coding.is_empty() && !coding.eq_ignore_ascii_case("chunked"))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        codings.push("chunked".to_string());
        headers.insert("Transfer-Encoding".to_string(), codings.join(", "));
    } else {
        remove_header(&mut headers, "Content-Length");
        headers.insert("Content-Length".to_string(), body.len().to_string());
    }

    write!(output, "{} {}\r\n", status_line.version, status_line.status)?;
    for (key, value) in headers {
        write!(output, "{key}: {value}\r\n")?;
    }
    output.write_all(b"\r\n")?;

    if use_chunked {
        if !body.is_empty() {
            write!(output, "{:x}\r\n", body.len())?;
            output.write_all(body)?;
            output.write_all(b"\r\n")?;
        }
        output.write_all(b"0\r\n\r\n")?;
    } else {
        output.write_all(body)?;
    }

    output.flush()
}

/// Logs a response-writing failure with request context.
pub fn log_writer_error(error: io::Error, context: &str) {
    eprintln!("[{context}] response I/O failed: {error}");
}

fn remove_header(headers: &mut BTreeMap<String, String>, name: &str) -> Option<String> {
    let existing = headers
        .keys()
        .find(|key| key.eq_ignore_ascii_case(name))
        .cloned()?;
    headers.remove(&existing)
}

fn insert_header(headers: &mut BTreeMap<String, String>, name: String, value: String) {
    remove_header(headers, &name);
    headers.insert(name, value);
}

fn contains_token(value: &str, expected: &str) -> bool {
    value
        .split(',')
        .map(str::trim)
        .any(|token| token.eq_ignore_ascii_case(expected))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serialize(
        version: HttpVersion,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> io::Result<Vec<u8>> {
        let mut output = Vec::new();
        let status_line = ResponseStatusLine {
            version,
            status: HttpStatusCode::Ok,
        };
        let headers = headers
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect();

        write_response(&mut output, &status_line, &headers, body)?;
        Ok(output)
    }

    #[test]
    fn writes_chunked_body_for_http_1_1() {
        let output = serialize(
            HttpVersion::Http1_1,
            &[("Transfer-Encoding", "chunked")],
            b"hello",
        )
        .unwrap();

        assert_eq!(
            output,
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n"
        );
    }

    #[test]
    fn uses_content_length_for_http_1_0_when_chunking_is_requested() {
        let output = serialize(
            HttpVersion::Http1_0,
            &[("Transfer-Encoding", "chunked")],
            b"hello",
        )
        .unwrap();

        assert_eq!(output, b"HTTP/1.0 200 OK\r\nContent-Length: 5\r\n\r\nhello");
    }

    #[test]
    fn corrects_content_length_before_writing() {
        let output =
            serialize(HttpVersion::Http1_1, &[("Content-Length", "999")], b"hello").unwrap();

        assert_eq!(output, b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello");
    }

    #[test]
    fn deduplicates_header_names_case_insensitively() {
        let output = serialize(
            HttpVersion::Http1_1,
            &[("Content-Length", "999"), ("content-length", "888")],
            b"hello",
        )
        .unwrap();
        let output = String::from_utf8(output).unwrap().to_ascii_lowercase();

        assert_eq!(output.matches("content-length:").count(), 1);
    }
}
