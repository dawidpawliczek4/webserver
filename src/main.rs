// Dawid Pawliczek, 347081
pub mod request;
pub mod response;
use crate::request::parse_request;
use crate::response::{send_error, send_response};
use std::io::BufReader;
use std::net::{TcpListener, TcpStream};
use std::time::Duration;
use std::{env, io};

fn handle_connection(stream: TcpStream, serve_path: &str) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(1)))?;

    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(&stream);

    loop {
        match parse_request(&mut reader) {
            Ok(req) => {
                if let Err(e) = send_response(&mut writer, &req, serve_path) {
                    eprintln!("write error: {e}");
                    break;
                }
                if req.connection_close {
                    break;
                }
            }
            Err(request::ParseError::Invalid) => {
                if let Err(e) = send_error(&mut writer, 501, "Not Implemented") {
                    eprintln!("write error: {e}");
                    break;
                }
            }
            Err(request::ParseError::Closed) => break,
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
