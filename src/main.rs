use flate2::Compression;
use flate2::write::GzEncoder;
use std::{
    env,
    io::prelude::*,
    net::{TcpListener, TcpStream},
};

const HTTP_STATUS_200_OK: &str = "HTTP/1.1 200 OK\r\n";
const HTTP_STATUS_201_CREATED: &str = "HTTP/1.1 201 Created\r\n";
const HTTP_STATUS_404_NOT_FOUND: &str = "HTTP/1.1 404 Not Found\r\n";
const HTTP_STATUS_500_INTERVAL_SERVER_ERROR: &str = "HTTP/1.1 500 Internal Server Error\r\n";
const HTTP_STATUS_501_NOT_IMPLEMENTED: &str = "HTTP/1.1 501 Not Implemented\r\n";

const ACCEPTED_ENCODINGS: [&str; 1] = ["gzip"];

fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut args: Vec<String> = env::args().collect();

    let mut request = [0_u8; 1024];
    let bytes = stream.read(&mut request).unwrap();
    let request = String::from_utf8_lossy(&request[..bytes]);
    let http_request: Vec<_> = request.lines().collect();

    let method: String;
    let path: String;
    let response: String;

    if http_request.len() > 0 {
        method = http_request[0]
            .split_whitespace()
            .next()
            .unwrap()
            .to_string();
        path = http_request[0]
            .split_whitespace()
            .nth(1)
            .unwrap()
            .to_string();
    } else {
        method = String::from("GET");
        path = String::from("/");
    }

    if path == "/" {
        response = format!("{}\r\n", HTTP_STATUS_200_OK);
    } else if path.contains("/echo/") {
        let text = path.split("/").last().unwrap();

        let mut available_encodings: Vec<String> = Vec::new();
        for line in &http_request {
            if line.starts_with("Accept-Encoding:") {
                let encodings = line.split(": ").last().unwrap().split(", ").into_iter();
                for encoding in encodings {
                    if ACCEPTED_ENCODINGS.contains(&encoding) {
                        available_encodings.push(encoding.to_string());
                    }
                }
            }
        }

        if available_encodings.len() > 0 {
            let encoding = available_encodings.join(", ");
            let mut content_length: usize = text.len();

            if encoding.contains("gzip") {
               let mut gz_encoder = GzEncoder::new(Vec::new(), Compression::default());
               gz_encoder.write_all(text.as_bytes()).unwrap();

               let compressed = gz_encoder.finish().unwrap();            
               content_length = compressed.len();

               let headers = format!(
                   "Content-Type: text/plain\r\nContent-Length: {}\r\nContent-Encoding: {}\r\n",
                   &content_length,
                   encoding
               );

               response = format!("{}{}\r\n", HTTP_STATUS_200_OK, headers);

               stream.write_all(response.as_bytes()).unwrap();
               stream.write_all(compressed.as_slice()).unwrap();

               return;
            } else {
                let headers = format!(
                    "Content-Type: text/plain\r\nContent-Length: {}\r\nContent-Encoding: {}\r\n",
                    &content_length,
                    encoding
                );

                response = format!("{}{}\r\n{}", HTTP_STATUS_200_OK, headers, text);
            }
        } else {
            let headers = format!(
                "Content-Type: text/plain\r\nContent-Length: {}\r\n",
                text.len()
            );

            response = format!("{}{}\r\n{}", HTTP_STATUS_200_OK, headers, text);
        }
    } else if path == "/user-agent" {
        let mut user_agent = String::new();
        for line in &http_request {
            if line.starts_with("User-Agent:") {
                user_agent = line.split(": ").last().unwrap().to_string();
            }
        }

        let headers = format!(
            "Content-Type: text/plain\r\nContent-Length: {}\r\n",
            user_agent.len()
        );

        response = format!("{}{}\r\n{}", HTTP_STATUS_200_OK, headers, user_agent);
    } else if path.contains("/files/") {
        let mut directory = String::from("/tmp");

        if args.len() == 3 && args[1] == "--directory" {
            directory = args.remove(2);
        }

        let file_name = path.split("/").last().unwrap();

        match method.as_str() {
            "GET" => match std::fs::read_to_string(format!("{}/{}", directory, file_name)) {
                Ok(file) => {
                    let headers = format!(
                        "Content-Type: application/octet-stream\r\nContent-Length: {}\r\n",
                        file.len()
                    );

                    response = format!("{}{}\r\n{}", HTTP_STATUS_200_OK, headers, file);
                }
                Err(_) => {
                    response = format!("{}\r\n", HTTP_STATUS_404_NOT_FOUND);
                }
            },
            "POST" => {
                let mut file =
                    std::fs::File::create(format!("{}/{}", directory, file_name)).unwrap();

                let mut content_length: usize = 0;

                for line in &http_request {
                    if line.starts_with("Content-Length:") {
                        content_length = line.split(": ").last().unwrap().parse::<usize>().unwrap();
                    }
                }

                let body = http_request.last().unwrap();
                let body = body.as_bytes();

                match file.write_all(body) {
                    Ok(_) => {
                        println!("Written {} bytes", content_length);

                        response = format!("{}\r\n", HTTP_STATUS_201_CREATED)
                    }
                    Err(err) => {
                        println!("Error: {}", err);

                        response = format!("{}\r\n", HTTP_STATUS_500_INTERVAL_SERVER_ERROR);
                    }
                }
            }
            _ => {
                response = format!("{}\r\n", HTTP_STATUS_501_NOT_IMPLEMENTED);
            }
        }
    } else {
        response = format!("{}\r\n", HTTP_STATUS_404_NOT_FOUND);
    }

    stream.write_all(response.as_bytes()).unwrap();

    return;
}
