use mio::Token;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use crate::data_structures::stream::Stream;

pub type Key = Arc<[u8]>;

pub enum Value {
    String(Vec<u8>),
    List(VecDeque<Vec<u8>>),
    Stream(Stream)
}

pub struct Entry {
    pub value: Value,
    pub expiry: Option<Instant>,
}

pub type DB = HashMap<Key, Entry>;
pub type Expiries = BinaryHeap<(Reverse<Instant>, Key)>;
pub type PubSub = HashMap<Vec<u8>, Vec<Token>>;
