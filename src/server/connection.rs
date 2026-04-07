use mio::net::TcpStream;
use slab::Slab;
use std::time::Instant;

use crate::types::Key;

pub struct Connection {
    pub stream: TcpStream,
    pub read_buffer: Vec<u8>,
    pub write_buffer: Vec<u8>,
    pub blocked: bool,
    pub block_key: Option<Key>,
    pub block_deadline: Option<Instant>,
    pub subscriptions: Vec<Vec<u8>>,
    pub is_pubsub: bool,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            read_buffer: Vec::new(),
            write_buffer: Vec::new(),
            blocked: false,
            block_key: None,
            block_deadline: None,
            subscriptions: Vec::new(),
            is_pubsub: false,
        }
    }
}

pub type ConnectionSlab = Slab<Connection>;
