use anyhow::{Result, Error};
use std::{
    fs,
    fs::File,
    io::{Write},
    net::{TcpListener, TcpStream},
    str, thread,
};
use flate2::{
    write::GzEncoder,
    Compression,
};

#[derive(Debug)]
struct HttpRequest {
    path: String,
    user_agent: Option<String>,
    accept_encoding: Option<String>,
}

impl Default for HttpRequest {
    fn default() -> Self {
        HttpRequest {
            user_agent: Default::default(),
            path: Default::default(),
            accept_encoding: Default::default(),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = if args.len() > 2 {
        args.windows(2)
            .find(|window| window[0] == "--directory")
            .map_or(String::new(), |window| window[1].to_owned())
    } else {
        String::new()
    };

    println!("Starting server with directory: {}", dir);

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let directory = dir.clone();
                let _handle = thread::spawn(move || {
                    println!("Connection established");
                    let mut request = [0_u8; 1024];
                    match stream.peek(&mut request) {
                        Ok(bytes) => {
                            let request_string = String::from_utf8_lossy(&request[..bytes]).into_owned();
                            println!("Received request: {}", request_string);
                            handle_connection(stream, &request_string, &directory);
                        }
                        Err(e) => {
                            println!("Failed to read from stream: {}", e);
                        }
                    }
                });
            }
            Err(e) => {
                println!("Error accepting connection: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream, request_string: &str, directory: &str) {
    println!("Handling connection with directory: {}", directory);

    let http_request = match parse_request(request_string) {
        Ok(req) => req,
        Err(e) => {
            println!("Failed to parse request: {}", e);
            return;
        }
    };

    let (first_line, remaining_lines) = match request_string.split_once("\r\n") {
        Some((f, r)) => (f, r),
        None => {
            println!("Incorrect request: {}", request_string);
            return;
        }
    };

    let (method, remaining) = match first_line.split_once(" ") {
        Some((m, r)) => (m, r),
        None => {
            println!("Incorrect request line: {}", first_line);
            return;
        }
    };

    println!("Parsed request - Method: {}, Path: {}", method, http_request.path);

    let ok_response = "HTTP/1.1 200 OK\r\n".to_string();
    let not_found_response = "HTTP/1.1 404 Not Found\r\n\r\n".to_string();
    let mut response_headers = String::new();
    let mut response_body = Vec::new();

    match method {
        "GET" => match http_request.path.as_str() {
            "/" => {
                response_headers.push_str(&ok_response);
                response_headers.push_str("\r\n");
            }
            path if path.starts_with("/echo/") => {
                let body = path.replace("/echo/", "");
                response_headers.push_str(&ok_response);
                response_headers.push_str(&format!(
                    "Content-Type: text/plain\r\nContent-Length: {}\r\n",
                    body.len()
                ));
                if let Some(accept_encoding) = http_request.accept_encoding {
                    if accept_encoding.contains("gzip") {
                        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                        encoder.write_all(body.as_bytes()).unwrap();
                        let compressed_body = encoder.finish().unwrap();
                        response_headers.push_str("Content-Encoding: gzip\r\n");
                        response_headers.push_str(&format!("Content-Length: {}\r\n", compressed_body.len()));
                        response_body = compressed_body;
                    } else {
                        response_body = body.into_bytes();
                    }
                } else {
                    response_body = body.into_bytes();
                }
                response_headers.push_str("\r\n");
            }
            "/user-agent" => {
                let body = http_request.user_agent.unwrap_or_default();
                response_headers.push_str(&ok_response);
                response_headers.push_str(&format!(
                    "Content-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
                    body.len()
                ));
                response_body = body.into_bytes();
            }
            path if path.starts_with("/files/") => {
                let file_name = path.replace("/files/", "");
                let mut file_path = directory.to_string();
                if !file_path.ends_with("/") {
                    file_path.push('/');
                }
                file_path.push_str(&file_name);
                println!("Looking for file at: {}", file_path);
                if file_path_exists(&file_path) {
                    match fs::read(&file_path) {
                        Ok(fc) => {
                            println!("File found, reading content.");
                            response_headers.push_str(&ok_response);
                            response_headers.push_str(&format!(
                                "Content-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n",
                                fc.len()
                            ));
                            response_body = fc;
                        }
                        Err(e) => {
                            println!("Failed to read file: {}", e);
                            response_headers.push_str(&not_found_response);
                        }
                    }
                } else {
                    println!("File not found at path: {}", file_path);
                    response_headers.push_str(&not_found_response);
                }
            }
            _ => {
                println!("Unknown GET path: {}", http_request.path);
                response_headers.push_str(&not_found_response);
            }
        },
        "POST" => {
            let contents = remaining_lines.split_once("\r\n\r\n").unwrap().1;
            let file_name = remaining.split_once(" ").unwrap().0;
            let file_name = file_name.strip_prefix("/files/").unwrap();
            let mut file_path = directory.to_owned();
            if !file_path.ends_with("/") {
                file_path.push('/');
            }
            file_path.push_str(file_name);
            println!("Writing file to: {}", file_path);
            let mut file = match File::create(&file_path) {
                Ok(f) => f,
                Err(e) => {
                    println!("Failed to create file: {}", e);
                    return;
                }
            };
            if let Err(e) = file.write_all(contents.as_bytes()) {
                println!("Failed to write to file: {}", e);
                return;
            }
            response_headers.push_str("HTTP/1.1 201 Created\r\nContent-Type: text/plain\r\nContent-Length: 0\r\n\r\n");
        }
        _ => {
            println!("Unknown method: {}", method);
            response_headers.push_str(&not_found_response);
        }
    }

    let response = [response_headers.as_bytes(), &response_body].concat();
    println!("Sending response: {}", String::from_utf8_lossy(&response));
    if let Err(e) = stream.write_all(&response) {
        println!("Failed to send response: {}", e);
    } else {
        println!("Response sent successfully");
    }
}

fn parse_request(req: &str) -> Result<HttpRequest, Error> {
    let content: Vec<&str> = req.lines().collect();
    let mut method_header = content[0].split_whitespace();
    let user_agent = content.iter().find(|&&line| line.starts_with("User-Agent: ")).unwrap_or(&"").replace("User-Agent: ", "");
    let accept_encoding = content.iter().find(|&&line| line.starts_with("Accept-Encoding: ")).unwrap_or(&"").replace("Accept-Encoding: ", "");
    let http_request = HttpRequest {
        path: String::from(method_header.nth(1).unwrap_or("")),
        user_agent: Some(user_agent),
        accept_encoding: Some(accept_encoding),
        ..Default::default()
    };
    Ok(http_request)
}

fn file_path_exists(path: &str) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => {
            println!("File metadata for path {}: {:?}", path, metadata);
            metadata.is_file()
        }
        Err(e) => {
            println!("Failed to get metadata for path {}: {}", path, e);
            false
        }
    }
}