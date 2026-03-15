#![allow(unused_imports)]
use std::{
    io::{Read, Write},
    net::TcpListener,
};

fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");

                let mut buffer = [0; 1024];

                loop {
                    let bytes_read = match stream.read(&mut buffer) {
                        Ok(0) => break, // connection closed
                        Ok(n) => n,
                        Err(_) => break,
                    };

                    let input = String::from_utf8_lossy(&buffer[..bytes_read]);
                    println!("received: {}", input);

                    if input.contains("PING") {
                        stream.write_all(b"+PONG\r\n").unwrap();
                    }
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}