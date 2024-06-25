use anyhow::{Result, Error};
use std::{
    fs,
    io::Write,
    net::{TcpListener, TcpStream},
    str, thread,
};
use std::fs::File;

#[derive(Debug)]
struct HttpRequest {
    path: String,
    user_agent: Option<String>,
}

impl Default for HttpRequest {
    fn default() -> Self {
        HttpRequest {
            user_agent: Default::default(),
            path: Default::default(),
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
            println!("Malformed request: {}", request_string);
            return;
        }
    };

    let (method, remaining) = match first_line.split_once(" ") {
        Some((m, r)) => (m, r),
        None => {
            println!("Malformed request line: {}", first_line);
            return;
        }
    };

    println!("Parsed request - Method: {}, Path: {}", method, http_request.path);

    let ok_response = "HTTP/1.1 200 OK\r\n\r\n".to_string();
    let not_found_response = "HTTP/1.1 404 Not Found\r\n\r\n".to_string();
    let response = match method {
        "GET" => match http_request.path.as_str() {
            "/" => ok_response.clone(),
            path if path.starts_with("/echo/") => {
                let body = path.replace("/echo/", "");
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                )
            }
            "/user-agent" => {
                let body = http_request.user_agent.unwrap_or_default();
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}\r\n",
                    body.len(),
                    body
                )
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
                            format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}\r\n",
                                fc.len(),
                                String::from_utf8(fc).expect("File content")
                            )
                        }
                        Err(e) => {
                            println!("Failed to read file: {}", e);
                            not_found_response.clone()
                        }
                    }
                } else {
                    println!("File not found at path: {}", file_path);
                    not_found_response.clone()
                }
            }
            _ => {
                println!("Unknown GET path: {}", http_request.path);
                not_found_response.clone()
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
            "HTTP/1.1 201 Created\r\nContent-Type: text/plain\r\nContent-Length: 0\r\n\r\n".to_string()
        }
        _ => {
            println!("Unknown method: {}", method);
            not_found_response.clone()
        }
    };

    println!("Sending response: {}", response);
    if let Err(e) = stream.write_all(response.as_bytes()) {
        println!("Failed to send response: {}", e);
    } else {
        println!("Response sent successfully");
    }
}

fn parse_request(req: &str) -> Result<HttpRequest, Error> {
    let content: Vec<&str> = req.lines().collect();
    let mut method_header = content[0].split_whitespace();
    let user_agent = content.iter().find(|&&line| line.starts_with("User-Agent: ")).unwrap_or(&"").replace("User-Agent: ", "");
    let http_request = HttpRequest {
        path: String::from(method_header.nth(1).unwrap_or("")),
        user_agent: Some(user_agent),
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