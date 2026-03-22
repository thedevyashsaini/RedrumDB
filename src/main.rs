use mio::net::TcpListener;
use mio::{Events, Interest, Poll, Token};
use slab::Slab;
use std::io::{Read, Write};

mod commands;
use commands::parse_command;
use std::collections::HashMap;

const SERVER: Token = Token(0);

fn main() -> std::io::Result<()> {
    println!("Starting Redis-like server on 127.0.0.1:6379");
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);

    let mut listener = TcpListener::bind("127.0.0.1:6379".parse().unwrap())?;

    poll.registry()
        .register(&mut listener, SERVER, Interest::READABLE)?;

    let mut connections = Slab::new();

    let mut db: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                SERVER => loop {
                    match listener.accept() {
                        Ok((mut stream, _addr)) => {
                            let entry = connections.vacant_entry();
                            let token: Token = Token(entry.key() + 1);

                            poll.registry()
                                .register(&mut stream, token, Interest::READABLE)?;

                            entry.insert((stream, Vec::new()));
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
                },

                token => {
                    let idx = token.0 - 1;

                    if let Some((stream, buffer)) = connections.get_mut(idx) {
                        let mut temp: [u8; 1024] = [0; 1024];

                        match stream.read(&mut temp) {
                            Ok(0) => {
                                connections.remove(idx);
                            }
                            Ok(n) => {
                                buffer.extend_from_slice(&temp[..n]);

                                println!(
                                    "Received: \r\n{}",
                                    std::str::from_utf8(buffer)
                                        .unwrap()
                                        .trim()
                                );

                                match parse_command(buffer) {
                                    Ok(command) => {
                                        println!("Command: {:?}", command.cmd_type);

                                        let response = command.process(&mut db).unwrap();
                                        let _ = stream.write_all(response.as_bytes());
                                        buffer.clear();
                                    }

                                    Err(_) => {
                                        let _ = stream.write_all(b"-ERR invalid RESP\r\n");
                                        buffer.clear();
                                    }
                                }
                            }
                            Err(e) => {
                                if e.kind() != std::io::ErrorKind::WouldBlock {
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