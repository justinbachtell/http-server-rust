use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};

fn main() {
    // create a listener on port 4221
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    // listen for incoming connections
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    // store the stream in a buffer
    let buf_reader = BufReader::new(&mut stream);

    // read the request line by line
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    // split the request line into parts
    let parts = http_request[0].split(" ").collect::<Vec<&str>>();
    let response: String;

    // match the request line
    match parts[1] {
        "/" => response = String::from("HTTP/1.1 200 OK\r\n\r\n"),
        _default => {
            // if endpoint is /echo
            if _default.contains("/echo") {
                let new_parts = _default.split("/").collect::<Vec<&str>>();
                response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    new_parts[2].len(),
                    new_parts[2]
                );
            // if endpoint is /user-agent
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
            // if endpoint is /files/{filename}
            } else if _default.contains("/files") {
                let new_parts = _default.split("/").collect::<Vec<&str>>();
                let filename = new_parts[2];
                let file = fs::read_to_string(filename).unwrap();

                // if file is found return the file content otherwise 404
                if file.len() > 0 {
                    response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                        file.len(),
                        file
                    );
                } else {
                    response = String::from("HTTP/1.1 404 Not Found\r\n\r\n");
                }
            // if endpoint is not found
            } else {
                response = String::from("HTTP/1.1 404 Not Found\r\n\r\n");
            }
        }
    };

    // write the response to the stream
    stream.write(response.as_bytes()).unwrap();
}
