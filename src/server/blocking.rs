use mio::{Interest, Poll, Token};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::io::Write;
use std::time::Instant;

use crate::server::connection::ConnectionSlab;
use crate::types::{Entry, Key, Value, DB};

pub fn wake_client(
    key: &[u8],
    db: &mut DB,
    blocked_queues: &mut HashMap<Key, VecDeque<Token>>,
    connections: &mut ConnectionSlab,
    poll: &mut Poll,
) -> std::io::Result<()> {
    if let Some(queue) = blocked_queues.get_mut(key) {
        while let Some(token) = queue.pop_front() {
            if let Some(conn) = connections.get_mut(token.0 - 1) {
                if !conn.blocked {
                    continue;
                }

                conn.blocked = false;
                conn.block_key = None;
                conn.block_deadline = None;

                if let Some(Entry {
                    value: Value::List(ref mut list),
                    ..
                }) = db.get_mut(key)
                {
                    if let Some(val) = list.pop_front() {
                        let mut res = Vec::with_capacity(val.len() + key.len() + 64);

                        write!(res, "*2\r\n")?;

                        write!(res, "${}\r\n", key.len())?;
                        res.extend_from_slice(key);
                        res.extend_from_slice(b"\r\n");

                        write!(res, "${}\r\n", val.len())?;
                        res.extend_from_slice(&val);
                        res.extend_from_slice(b"\r\n");

                        let was_empty = conn.write_buffer.is_empty();
                        conn.write_buffer.extend_from_slice(&res);

                        if was_empty {
                            poll.registry().reregister(
                                &mut conn.stream,
                                token,
                                Interest::READABLE.add(Interest::WRITABLE),
                            )?;
                        }
                    }
                }

                break;
            }
        }
    }

    Ok(())
}

pub fn cleanup_blocked(
    connections: &mut ConnectionSlab,
    blocked_timeouts: &mut BinaryHeap<(Reverse<Instant>, Token)>,
    poll: &mut Poll,
) {
    let now = Instant::now();

    while let Some((Reverse(t), token)) = blocked_timeouts.peek().cloned() {
        if t > now {
            break;
        }

        blocked_timeouts.pop();

        if let Some(conn) = connections.get_mut(token.0 - 1) {
            if conn.blocked {
                conn.blocked = false;
                conn.block_key = None;
                conn.block_deadline = None;

                let was_empty = conn.write_buffer.is_empty();

                conn.write_buffer.extend_from_slice(b"*-1\r\n");

                if was_empty {
                    poll.registry()
                        .reregister(
                            &mut conn.stream,
                            token,
                            Interest::READABLE.add(Interest::WRITABLE),
                        )
                        .unwrap();
                }
            }
        }
    }
}
