use anyhow::Error;
use std::{
    env, fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    str, thread,
};

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    version: String,
    host: Option<String>,
    user_agent: Option<String>,
}

impl Default for HttpRequest {
    fn default() -> Self {
        HttpRequest {
            user_agent: Default::default(),
            host: Default::default(),
            method: Default::default(),
            version: Default::default(),
            path: Default::default(),
        }
    }
}

fn main() {
    // create a listener on port 4221
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    // listen for incoming connections
    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                thread::spawn(|| handle_connection(_stream));
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    println!("Connection established");

    // create a buffer to store the stream
    let mut buffer = [0; 1024];

    let ok_response = "HTTP/1.1 200 OK\r\n\r\n".to_string();

    let not_found_response = "HTTP/1.1 404 Not Found\r\n\r\n".to_string();

    // read the stream into the buffer
    match stream.read(&mut buffer) {
        Ok(_n) => {
            println!("Request received");
            let request = str::from_utf8(&buffer).unwrap();
            let http_request = parse_request(&request).unwrap();
            let mut response = not_found_response.clone();

            println!("Path: {:?}", http_request.user_agent);

            if http_request.path == "/" {
                response = ok_response.clone();
            } else if http_request.path.starts_with("/echo") {
                let body = http_request.path.replace("/echo/", "");
                response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
            } else if http_request.path.starts_with("/user-agent") {
                let body = http_request.user_agent.unwrap();
                if http_request.method == "GET" {
                    response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}\r\n",
                        body.len(),
                        body
                    )
                }
            } else if http_request.path.starts_with("/files") {
                let file_name = http_request.path.replace("/files/", "");
                let env_args: Vec<String> = env::args().collect();
                let mut dir = env_args[2].clone();
                dir.push_str(&file_name);
                let file = fs::read(dir);
                match file {
                    Ok(fc) => {
                        response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}\r\n", fc.len(), String::from_utf8(fc).expect("File content"));
                    }
                    Err(..) => response = not_found_response.clone(),
                }
            }
            match stream.write(response.as_bytes()) {
                Ok(_) => println!("Ok"),
                Err(e) => println!("err: {}", e),
            }
        }
        Err(e) => println!("Fail connect: {}", e),
    }
}

fn parse_request(req: &str) -> Result<HttpRequest, Error> {
    let content: Vec<&str> = req.lines().collect();
    let mut method_header = content[0].split_whitespace();
    let host = content[1].replace("Host: ", "");
    let user_agent = content[2].replace("User-Agent: ", "");
    let http_request = HttpRequest {
        method: String::from(method_header.next().unwrap()),
        path: String::from(method_header.next().unwrap()),
        version: String::from(method_header.next().unwrap()),
        host: Some(host),
        user_agent: Some(user_agent),
        ..Default::default()
    };
    Ok(http_request)
}