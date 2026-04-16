<img width="1125" height="375" alt="High-performance in-memory datastore in Rust(1)" src="https://github.com/user-attachments/assets/bb988174-d428-4ec0-a2fb-02e284461c94" />


![Rust](https://img.shields.io/badge/language-Rust-orange?style=for-the-badge&logo=rust)
![Event Driven](https://img.shields.io/badge/architecture-event--driven-black?style=for-the-badge)
![Zero Copy](https://img.shields.io/badge/parsing-zero--copy-blue?style=for-the-badge)
![No Alloc](https://img.shields.io/badge/memory-allocation--aware-red?style=for-the-badge)

> no ai. just docs, articles, code, and time.

RedrumDB is an in-memory datastore implemented in Rust.

The project focuses on building core components from scratch to better understand the design and performance characteristics of systems like [Redis](https://github.com/redis/redis).

<h3>Features</h3>

- key-value store (strings, lists, streams)
- RESP protocol support
- pub/sub
- blocking operations (BLPOP)
- key expiries
- non-blocking I/O


<h3>Architecture</h3>

The server runs on a single-threaded, event-driven loop using `mio`, with slab-based connection management and non-blocking I/O.

Requests are parsed using a low-copy RESP parser, decoding command names and arguments directly from the connection buffer before dispatch.

Core components:

- **event loop + connections**
  - `mio`-based loop with explicit readable/writable interest switching  
  - slab + token model for stable connection indexing  
  - backpressure-aware writes with per-connection output buffers  

- **command execution**
  - command dispatch table for normalized O(1)-style handler lookup  
  - centralized handling of pub/sub mode constraints  

- **memory + data model**
  - in-memory keyspace with typed values (string, list, stream)  
  - shared key storage using `Arc<[u8]>` to reduce duplication  

- **scheduling + time**
  - expiry engine using a min-heap (`BinaryHeap<Reverse<Instant>>`)  
  - incremental cleanup bounded per loop to avoid latency spikes  

- **blocking + async behavior**
  - blocking list operations (BLPOP) with per-key wait queues  
  - timeout scheduling integrated into the main event loop  

- **streams**
  - custom **[listpack](https://github.com/antirez/listpack/blob/master/listpack.md)** implementation for compact storage  
  - **radix tree** with prefix compression for ordered indexing  
  - Redis-like stream ID semantics (`*`, `ms-*`, explicit IDs)

Design prioritizes predictable latency, minimal allocations, and explicit control over execution.

<h3>running</h3>

```bash
git clone https://github.com/thedevyashsaini/redrumdb
cd redrumdb
cargo run
```

Connect using:

```bash
redis-cli -p 6379
```
