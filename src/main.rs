// Dawid Pawliczek, 347081
use std::{env, fs, io};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::time::Duration;
use crate::ParseError::{Closed, Invalid};

enum ParseError {
    Invalid, // return 501
    Closed // eof, timeout -> silent close
}

struct Request {
    url_path: String,
    host: String,
    connection_close: bool
}

fn content_type_for(path: &Path) -> &'static str {
    path
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| match ext {
            "html" => "text/html; charset=utf-8",
            "txt"  => "text/plain; charset=utf-8",
            "css"  => "text/css",
            "jpg" | "jpeg" => "image/jpeg",
            "png"  => "image/png",
            "pdf"  => "application/pdf",
            _      => "application/octet-stream",
        })
        .unwrap_or("application/octet-stream")
}

fn handle_request(writer: &mut TcpStream, request: &Request, serve_path: &str) -> io::Result<()> {
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
        body.len(), body
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

fn send_error(writer: &mut TcpStream, status: u16, message: &str) -> io::Result<()> {
    let body = format!("{status}: {message}");
    let response = format!(
        "HTTP/1.1 {status} {message}\r\n\
        Content-Type: text/html; charset=utf-8\r\n\
        Content-Length: {}\r\n\
        \r\n\
        {}",
        body.len(), body);
    writer.write_all(response.as_bytes())
}

fn is_timeout(e: &io::Error) -> bool { e.kind() == io::ErrorKind::TimedOut || e.kind() == io::ErrorKind::WouldBlock  }

fn parse_url_path(line: &str) -> Option<String> {
    let mut parts = line.trim_end_matches(&['\r', '\n'][..]).split(' ');
    let method = parts.next()?;
    let path = parts.next()?;
    let version = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    if method != "GET" || !version.starts_with("HTTP/") {
        return None;
    }
    Some(path.to_string())
}
fn parse_host(line: &str) -> Option<String> {
    let (key, value) = line.split_once(":")?;
    if !key.eq_ignore_ascii_case("Host") {
        return None;
    }
    let value = value.trim();
    match value.split_once(":") {
        Some((host, _port)) => Some(host.to_string()),
        None => Some(value.to_string())
    }
}
fn parse_connection(line: &str) -> Option<bool> {
    let (key, value) = line.split_once(":")?;
    if !key.eq_ignore_ascii_case("Connection") {
        return None;
    }
    let value = value.trim();

    if value.eq_ignore_ascii_case("close") {
        Some(true)
    } else if value.eq_ignore_ascii_case("keep-alive") {
        Some(false)
    } else {
        None
    }
}

fn parse_request(reader: &mut BufReader<&TcpStream>) -> Result<Request, ParseError> {

    let mut url_path = None;
    let mut host = None;
    let mut connection_close = false;
    let mut got_any_line = false;

    loop {

        let mut request_line = String::new();
        match reader.read_line(&mut request_line) {
            Ok(0) => {
                if !got_any_line {
                    return Err(Closed);
                }
                break;
            }
            Ok(_) if request_line.trim().is_empty() => break,
            Ok(_) => {
                got_any_line = true;
                if let Some(some_url_path) = parse_url_path(&request_line) {
                    url_path = Some(some_url_path)
                } else if let Some(some_host) = parse_host(&request_line) {
                    host = Some(some_host)
                } else if let Some(some_connection_close) = parse_connection(&request_line) {
                    connection_close = some_connection_close
                }
            }
            Err(e) if is_timeout(&e) => return Err(Closed),
            Err(_) => return Err(Invalid)
        }
    }

    if let Some(url_path) = url_path && let Some(host) = host {
        Ok(Request { url_path, host, connection_close })
    } else {
        Err(Invalid)
    }
}


fn handle_connection(stream: TcpStream, serve_path: &str) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(1)))?;

    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(&stream);

    loop {
        match parse_request(&mut reader) {
            Ok(req) => {
                if let Err(e) = handle_request(&mut writer, &req, serve_path) {
                    eprintln!("write error: {e}");
                    break;
                }
                if req.connection_close {
                    break;
                }
            }
            Err(Invalid) => {
                if let Err(e) = send_error(&mut writer, 501, "Not Implemented") {
                    eprintln!("write error: {e}");
                    break;
                }
            }
            Err(Closed) => break,
        }
    }

    Ok(())
}

fn main() -> io::Result<()> {

    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("usage: webserver <port> <path/to/files>");
        std::process::exit(1);
    }

    let port: u16 = match args[1].parse() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("error: invalid port `{}`", args[1]);
            std::process::exit(1);
        }
    };

    if !std::path::Path::new(&args[2]).is_dir() {
        eprintln!("error: `{}` is not a directory", args[2]);
        std::process::exit(1);
    }

    let listener: TcpListener = TcpListener::bind(format!("0.0.0.0:{}", port))?;

    for stream in listener.incoming() {
        let stream = stream?;
        handle_connection(stream, &args[2])?;
    }

    Ok(())
}
