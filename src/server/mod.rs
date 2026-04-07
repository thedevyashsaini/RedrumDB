mod blocking;
pub mod connection;
mod expiry;

use mio::{net::TcpListener, Events, Interest, Poll, Token};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Instant;

use crate::commands::{command_table, normalize_upper, parse_command, Action, Context};
use crate::server::connection::{Connection, ConnectionSlab};
use crate::types::{Expiries, Key, PubSub, DB};

const SERVER: Token = Token(0);

pub fn run() -> std::io::Result<()> {
    println!("Starting Redis-like server on 127.0.0.1:6379");
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);

    let mut listener = TcpListener::bind("127.0.0.1:6379".parse().unwrap())?;

    poll.registry()
        .register(&mut listener, SERVER, Interest::READABLE)?;

    let mut connections: ConnectionSlab = slab::Slab::new();

    let mut db: DB = HashMap::new();
    let mut expiries: Expiries = BinaryHeap::new();
    let mut blocked_queues: HashMap<Key, VecDeque<Token>> = HashMap::new();
    let mut blocked_timeouts: BinaryHeap<(Reverse<Instant>, Token)> = BinaryHeap::new();
    let mut pubsub: PubSub = HashMap::new();
    let mut actions: Vec<Action> = Vec::new();

    let table = command_table();

    loop {
        expiry::cleanup_expired(&mut db, &mut expiries);
        blocking::cleanup_blocked(&mut connections, &mut blocked_timeouts, &mut poll);

        let timeout = blocked_timeouts.peek().map(|(Reverse(t), _)| {
            let now = Instant::now();
            if *t <= now {
                std::time::Duration::from_millis(0)
            } else {
                *t - now
            }
        });

        poll.poll(&mut events, timeout)?;

        for event in events.iter() {
            match event.token() {
                SERVER => loop {
                    match listener.accept() {
                        Ok((mut stream, _addr)) => {
                            let entry = connections.vacant_entry();
                            let token: Token = Token(entry.key() + 1);

                            poll.registry()
                                .register(&mut stream, token, Interest::READABLE)?;

                            entry.insert(Connection::new(stream));
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

                    let mut conn = match connections.try_remove(idx) {
                        Some(c) => c,
                        None => continue,
                    };

                    let mut should_remove = false;
                    let mut wake_key: Option<Vec<u8>> = None;

                    if event.is_readable() {
                        if conn.blocked {
                            connections.insert(conn);
                            continue;
                        }

                        let mut temp = [0u8; 1024];

                        match conn.stream.read(&mut temp) {
                            Ok(0) => {
                                should_remove = true;
                            }
                            Ok(n) => {
                                conn.read_buffer.extend_from_slice(&temp[..n]);

                                match parse_command(&conn.read_buffer) {
                                    Ok(command) => {
                                        let mut temp = [0u8; 32];
                                        let normalized = normalize_upper(command.name, &mut temp);

                                        let is_empty = conn.write_buffer.is_empty();

                                        let mut ctx = Context {
                                            db: &mut db,
                                            expiries: &mut expiries,
                                            pubsub: &mut pubsub,
                                            subscriptions: &mut conn.subscriptions,
                                            is_pubsub: &mut conn.is_pubsub,
                                            token,
                                            actions: &mut actions,
                                        };

                                        match table.get(normalized) {
                                            Some(handler) => {
                                                match (handler)(&command.args, &mut ctx) {
                                                    Ok(bytes) => {
                                                        conn.write_buffer.extend_from_slice(&bytes);

                                                        if normalized == b"LPUSH"
                                                            || normalized == b"RPUSH"
                                                        {
                                                            if let Some(key) = command.args.get(0) {
                                                                wake_key = Some(key.to_vec());
                                                            }
                                                        }
                                                    }
                                                    Err(bytes) => {
                                                        if bytes == b"__BLOCK__" {
                                                            conn.blocked = true;

                                                            let key: Key =
                                                                Arc::from(command.args[0]);
                                                            conn.block_key = Some(key.clone());

                                                            let timeout = std::str::from_utf8(
                                                                command.args[1],
                                                            )
                                                            .unwrap()
                                                            .parse::<f64>()
                                                            .unwrap();

                                                            if timeout > 0.0 {
                                                                let deadline = Instant::now()
                                                                + std::time::Duration::from_millis(
                                                                    (timeout * 1000.0) as u64,
                                                                );

                                                                conn.block_deadline =
                                                                    Some(deadline);
                                                                blocked_timeouts.push((
                                                                    Reverse(deadline),
                                                                    token,
                                                                ));
                                                            }

                                                            blocked_queues
                                                                .entry(key.clone())
                                                                .or_insert_with(VecDeque::new)
                                                                .push_back(token);

                                                            connections.insert(conn);
                                                            continue;
                                                        } else {
                                                            conn.write_buffer
                                                                .extend_from_slice(&bytes);
                                                        }
                                                    }
                                                }
                                            }
                                            None => {
                                                conn.write_buffer
                                                    .extend_from_slice(b"-ERR unknown command\r\n");
                                            }
                                        }

                                        while let Some(action) = &actions.pop() {
                                            match action {
                                                Action::Publish { channel, message } => {
                                                    if let Some(subs) = pubsub.get(&channel[..]) {
                                                        for &token in subs {
                                                            if let Some(sub_conn) =
                                                                connections.get_mut(token.0 - 1)
                                                            {
                                                                let mut res = Vec::new();

                                                                write!(res, "*3\r\n")?;
                                                                write!(res, "$7\r\nmessage\r\n")?;

                                                                write!(
                                                                    res,
                                                                    "${}\r\n",
                                                                    channel.len()
                                                                )?;
                                                                res.extend_from_slice(&channel);
                                                                res.extend_from_slice(b"\r\n");

                                                                write!(
                                                                    res,
                                                                    "${}\r\n",
                                                                    message.len()
                                                                )?;
                                                                res.extend_from_slice(&message);
                                                                res.extend_from_slice(b"\r\n");

                                                                let was_empty = sub_conn
                                                                    .write_buffer
                                                                    .is_empty();
                                                                sub_conn
                                                                    .write_buffer
                                                                    .extend_from_slice(&res);

                                                                if was_empty {
                                                                    poll.registry().reregister(
                                                                        &mut sub_conn.stream,
                                                                        token,
                                                                        Interest::READABLE.add(
                                                                            Interest::WRITABLE,
                                                                        ),
                                                                    )?;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        if is_empty {
                                            poll.registry().reregister(
                                                &mut conn.stream,
                                                token,
                                                Interest::READABLE.add(Interest::WRITABLE),
                                            )?;
                                        }

                                        conn.read_buffer.clear();
                                    }
                                    Err(_) => {
                                        let is_empty = conn.write_buffer.is_empty();
                                        conn.write_buffer
                                            .extend_from_slice(b"-ERR invalid RESP\r\n");

                                        if is_empty {
                                            poll.registry().reregister(
                                                &mut conn.stream,
                                                token,
                                                Interest::READABLE.add(Interest::WRITABLE),
                                            )?;
                                        }

                                        conn.read_buffer.clear();
                                    }
                                }
                            }
                            Err(e) => {
                                if e.kind() != std::io::ErrorKind::WouldBlock {
                                    should_remove = true;
                                }
                            }
                        }
                    }

                    if event.is_writable() {
                        while !conn.write_buffer.is_empty() {
                            match conn.stream.write(&conn.write_buffer) {
                                Ok(0) => break,
                                Ok(n) => {
                                    conn.write_buffer.drain(..n);
                                }
                                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                                Err(_) => {
                                    should_remove = true;
                                    break;
                                }
                            }
                        }

                        if conn.write_buffer.is_empty() {
                            poll.registry().reregister(
                                &mut conn.stream,
                                token,
                                Interest::READABLE,
                            )?;
                        }
                    }

                    if !should_remove {
                        connections.insert(conn);
                    }

                    if let Some(key) = wake_key {
                        blocking::wake_client(
                            &key,
                            &mut db,
                            &mut blocked_queues,
                            &mut connections,
                            &mut poll,
                        )?;
                    }
                }
            }
        }
    }
}
