use mio::net::TcpListener;
use mio::{Events, Interest, Poll, Token};
use slab::Slab;
use std::io::{Read, Write};

const SERVER: Token = Token(0);

fn main() -> std::io::Result<()> {
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
                    // Accept new clients
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
                                let _ = stream.write_all(b"+PONG\r\n");
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
