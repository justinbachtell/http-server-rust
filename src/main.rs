use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

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

    let request = http_request[0].split(" ").collect::<Vec<&str>>();
    let http_response = match request[1] {
        "/" => "200 OK",
        _ => "404 NOT FOUND",
    };

    let response = format!("HTTP/1.1 {}\r\n\r\n", http_response);:

    stream.write_all(response.as_bytes()).unwrap();
}
