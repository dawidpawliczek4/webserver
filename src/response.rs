use crate::request::Request;
use std::fs::File;
use std::io::Write;
use std::net::TcpStream;
use std::path::Path;
use std::{fs, io};

pub fn send_response(
    writer: &mut TcpStream,
    request: &Request,
    serve_path: &str,
) -> io::Result<()> {
    let domain_root = Path::new(serve_path).join(&request.host);

    let canonical_root = match fs::canonicalize(&domain_root) {
        Ok(p) => p,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return send_error(writer, 404, "Not Found");
        }
        Err(_) => return send_error(writer, 403, "Forbidden"),
    };

    let relative = request.url_path.trim_start_matches('/');
    let candidate = domain_root.join(relative);

    let canonical = match fs::canonicalize(&candidate) {
        Ok(p) => p,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return send_error(writer, 404, "Not Found");
        }
        Err(_) => return send_error(writer, 403, "Forbidden"),
    };

    if !canonical.starts_with(&canonical_root) {
        return send_error(writer, 403, "Forbidden");
    }

    if canonical.is_dir() {
        let mut location = request.url_path.clone();
        if !location.ends_with('/') {
            location.push('/');
        }
        location.push_str("index.html");
        send_redirect(writer, &location)
    } else {
        send_file(writer, &canonical)
    }
}

pub fn send_error(writer: &mut TcpStream, status: u16, message: &str) -> io::Result<()> {
    let body = format!("{status}: {message}");
    let response = format!(
        "HTTP/1.1 {status} {message}\r\n\
        Content-Type: text/html; charset=utf-8\r\n\
        Content-Length: {}\r\n\
        \r\n\
        {}",
        body.len(),
        body
    );
    writer.write_all(response.as_bytes())
}

fn send_file(writer: &mut TcpStream, path: &Path) -> io::Result<()> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let mime = content_type_for(path);

    let headers = format!(
        "HTTP/1.1 200 OK\r\n\
        Content-Type: {mime}\r\n\
        Content-Length: {size}\r\n\
        \r\n"
    );
    writer.write_all(headers.as_bytes())?;
    io::copy(&mut file, writer)?;
    Ok(())
}

fn send_redirect(writer: &mut TcpStream, location: &str) -> io::Result<()> {
    let body = format!(
        "<html><body>301 Moved Permanently: <a href=\"{location}\">{location}</a></body></html>"
    );
    let response = format!(
        "HTTP/1.1 301 Moved Permanently\r\n\
        Location: {location}\r\n\
        Content-Type: text/html; charset=utf-8\r\n\
        Content-Length: {}\r\n\
        \r\n\
        {}",
        body.len(),
        body
    );
    writer.write_all(response.as_bytes())
}

fn content_type_for(path: &Path) -> &'static str {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| match ext {
            "html" => "text/html; charset=utf-8",
            "txt" => "text/plain; charset=utf-8",
            "css" => "text/css",
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "pdf" => "application/pdf",
            _ => "application/octet-stream",
        })
        .unwrap_or("application/octet-stream")
}
