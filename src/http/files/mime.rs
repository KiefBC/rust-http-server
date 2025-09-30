const TEXT_EXTENSIONS: &[&str] = &["txt", "html", "json", "js", "css", "xml"];

/// Returns true if the given file extension is a text extension.
pub fn is_text_extension(extension: &str) -> bool {
    TEXT_EXTENSIONS.contains(&extension.to_lowercase().as_str())
}

/// Returns the MIME type for a given file extension.
pub fn mime_type_from_extension(extension: &str) -> &str {
    match extension.to_lowercase().as_str() {
        "txt" => "text/plain",
        "html" => "text/html",
        "json" => "application/json",
        "js" => "application/javascript",
        "css" => "text/css",
        "xml" => "application/xml",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "jpg" => "image/jpeg",
        "png" => "image/png",
        "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        _ => "application/octet-stream",
    }
}
