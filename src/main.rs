use mio::net::TcpListener;
use mio::{Events, Interest, Poll, Token};
use slab::Slab;
use std::io::{Read, Write};

mod resp;
use resp::{parse_command};

const SERVER: Token = Token(0);

fn main() -> std::io::Result<()> {
    println!("Starting Redis-like server on 127.0.0.1:6379");
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);

    let mut listener = TcpListener::bind("127.0.0.1:6379".parse().unwrap())?;

    // Register listener
    poll.registry()
        .register(&mut listener, SERVER, Interest::READABLE)?;

    let mut connections = Slab::new();

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                SERVER => {
                    loop {
                        match listener.accept() {
                            Ok((mut stream, _addr)) => {
                                let entry = connections.vacant_entry();
                                let token: Token = Token(entry.key() + 1);

                                poll.registry()
                                    .register(&mut stream, token, Interest::READABLE)?;

                                entry.insert(stream);
                            }
                            Err(e) => {
                                if e.kind() == std::io::ErrorKind::WouldBlock {
                                    break;
                                } else {
                                    eprintln!("accept error: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                }

                token => {
                    let idx = token.0 - 1;

                    if let Some(stream) = connections.get_mut(idx) {
                        let mut buffer: [u8; 1024] = [0; 1024];

                        match stream.read(&mut buffer) {
                            Ok(0) => {
                                connections.remove(idx);
                            }
                            Ok(_n) => {
                                println!(
                                    "Received: {}",
                                    std::str::from_utf8(&buffer)
                                        .unwrap()
                                        .trim()
                                        .replace("\r\n", "\\r\\n")
                                );

                                if let Some((cmd, _consumed)) = parse_command(&buffer) {
                                    println!("Parsed command: {:?}", cmd);

                                    let response = handle_command(cmd);
                                    let _ = stream.write_all(response.as_bytes());
                                } else {
                                    let _ = stream.write_all(b"-ERR invalid RESP\r\n");
                                }
                            }
                            Err(e) => {
                                if e.kind() == std::io::ErrorKind::WouldBlock {
                                } else {
                                    connections.remove(idx);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn handle_command(cmd: Vec<String>) -> String {
    if cmd.is_empty() {
        return "-ERR empty command\r\n".to_string();
    }

    match cmd[0].to_uppercase().as_str() {
        "PING" => {
            if cmd.len() > 1 {
                format!("${}\r\n{}\r\n", cmd[1].len(), cmd[1])
            } else {
                "+PONG\r\n".to_string()
            }
        }
        "ECHO" => {
            if cmd.len() < 2 {
                "-ERR wrong number of arguments\r\n".to_string()
            } else {
                format!("${}\r\n{}\r\n", cmd[1].len(), cmd[1])
            }
        }
        _ => "-ERR unknown command\r\n".to_string(),
    }
}