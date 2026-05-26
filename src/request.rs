use crate::request::ParseError::{Closed, Invalid};
use std::io;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;

pub enum ParseError {
    Invalid, // return 501
    Closed,  // eof, timeout -> silent close
}

pub struct Request {
    pub url_path: String,
    pub host: String,
    pub connection_close: bool,
}

pub fn parse_request(reader: &mut BufReader<&TcpStream>) -> Result<Request, ParseError> {
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
            Err(_) => return Err(Invalid),
        }
    }

    if let Some(url_path) = url_path
        && let Some(host) = host
    {
        Ok(Request {
            url_path,
            host,
            connection_close,
        })
    } else {
        Err(Invalid)
    }
}

fn is_timeout(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::TimedOut || e.kind() == io::ErrorKind::WouldBlock
}

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
        None => Some(value.to_string()),
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
