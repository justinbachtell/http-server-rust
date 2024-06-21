use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();
    let parts = http_request[0].split(" ").collect::<Vec<&str>>();
    let response: String;
    match parts[1] {
        "/" => response = String::from("HTTP/1.1 200 OK\r\n\r\n"),
        _default => {
            if _default.contains("/echo") {
                let new_parts = _default.split("/").collect::<Vec<&str>>();
                response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    new_parts[2].len(),
                    new_parts[2]
                );
            } else if _default.contains("/user-agent") {
                let header = http_request
                    .iter()
                    .find(|line| line.contains("User-Agent"))
                    .unwrap();
                let new_parts = header.split(" ").collect::<Vec<&str>>();
                response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    new_parts[1].len(),
                    new_parts[1]
                );
            } else {
                response = String::from("HTTP/1.1 404 Not Found\r\n\r\n");
            }
        }
    };
    stream.write(response.as_bytes()).unwrap();
}
