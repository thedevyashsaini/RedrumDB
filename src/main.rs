use crate::data_structures::listpack::{Listpack, ListpackValueRef};

mod commands;
mod data_structures;
mod server;
mod types;

// fn main() -> std::io::Result<()> {
//     server::run()
// }

// fn main() {
//     let mut lp = Listpack::new();
//
//     lp.append(b"hello").unwrap();
//     lp.append(b"123").unwrap();
//     lp.append(b"world").unwrap();
//     lp.append(b"-45").unwrap();
//
//     println!("len = {}", lp.len());
//
//     for (i, v) in lp.iter().enumerate() {
//         match v {
//             ListpackValueRef::String(s) => {
//                 println!("{}: str = {}", i, std::str::from_utf8(s).unwrap());
//             }
//             ListpackValueRef::Int(x) => {
//                 println!("{}: int = {}", i, x);
//             }
//         }
//     }
// }

fn main() {
    let mut lp = Listpack::new();
    let mut vec: Vec<Vec<u8>> = Vec::new();

    let values = (0..10_000)
        .map(|i| format!("val{}", i).into_bytes())
        .collect::<Vec<_>>();

    for v in &values {
        lp.append(v).unwrap();
        vec.push(v.clone());
    }

    // ---- Listpack ----
    let lp_bytes = lp.total_bytes();

    // ---- Vec<Vec<u8>> ----

    // 1. Payload (actual stored bytes)
    let payload: usize = vec.iter().map(|v| v.len()).sum();

    // 2. Allocated buffers (capacity of each inner Vec)
    let buffers: usize = vec.iter().map(|v| v.capacity()).sum();

    // 3. Vec struct overhead (ptr, len, cap per element)
    let vec_struct_overhead = vec.capacity() * std::mem::size_of::<Vec<u8>>();

    // 4. Outer Vec allocation
    let outer_vec_overhead = vec.capacity() * std::mem::size_of::<Vec<u8>>();

    // 5. Alignment padding (rough estimate)
    let alignment_overhead: usize = vec.iter().map(|v| v.capacity() - v.len()).sum();

    let total_vec_memory = buffers + vec_struct_overhead + outer_vec_overhead;

    println!("--- Listpack ---");
    println!("Total bytes: {}", lp_bytes);

    println!("\n--- Vec<Vec<u8>> ---");
    println!("Payload bytes: {}", payload);
    println!("Allocated buffers: {}", buffers);
    println!("Vec struct overhead: {}", vec_struct_overhead);
    println!("Outer Vec overhead: {}", outer_vec_overhead);
    println!("Alignment waste (est): {}", alignment_overhead);
    println!("Estimated total: {}", total_vec_memory);
}
