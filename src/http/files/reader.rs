use super::{
    mime::is_text_extension,
    types::{FileReadError, FileReadRequest, FileReadResult},
};
use crate::http::writer::HttpBody;
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};

/// Defines a trait for reading files.
trait FileReader {
    /// Reads the file and returns its content as an HttpBody.
    fn read(&self) -> Result<HttpBody, FileReadError>;
}

/// Represents a full file reader.
pub struct FullFileReader {
    /// Path to the file being read.
    path: PathBuf,
}

impl FileReader for FullFileReader {
    fn read(&self) -> Result<HttpBody, FileReadError> {
        let read_bytes = fs::read(&self.path).map_err(FileReadError::NotFound)?;
        let file_ext = self.path.extension().and_then(|ext| ext.to_str());
        match file_ext {
            Some(ext) => {
                if is_text_extension(ext) {
                    match String::from_utf8(read_bytes) {
                        Ok(text) => Ok(HttpBody::Text(text)),
                        Err(e) => Ok(HttpBody::Binary(e.into_bytes())),
                    }
                } else {
                    Ok(HttpBody::Binary(read_bytes))
                }
            }
            None => Ok(HttpBody::Binary(read_bytes)),
        }
    }
}

/// Reads a file with range support and returns metadata
pub fn read_file_with_range(request: FileReadRequest) -> Result<FileReadResult, FileReadError> {
    match request {
        FileReadRequest::Full(path) => {
            let file_reader = FullFileReader { path };
            let body = file_reader.read()?;
            let total_size = body.byte_len() as u64;
            
            Ok(FileReadResult {
                body,
                total_size,
                range: None,
            })
        }
        FileReadRequest::Range(path, range) => {
            let metadata = fs::metadata(&path).map_err(FileReadError::IoError)?;
            let file_size = metadata.len();

            if file_size == 0 {
                return Err(FileReadError::InvalidRange);
            }

            let start = range.start;
            let end = range.end.unwrap_or(file_size - 1);

            if start > end || end >= file_size {
                return Err(FileReadError::InvalidRange);
            }

            let mut file = File::open(&path).map_err(FileReadError::IoError)?;
            file.seek(SeekFrom::Start(start))
                .map_err(FileReadError::IoError)?;
            let mut buffer = vec![0; (end - start + 1) as usize];
            file.read_exact(&mut buffer)
                .map_err(FileReadError::IoError)?;

            Ok(FileReadResult {
                body: HttpBody::Binary(buffer),
                total_size: file_size,
                range: Some((start, end)),
            })
        }
    }
}
