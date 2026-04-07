use std::cmp::Reverse;
use std::time::Instant;

use crate::types::{Entry, Expiries, DB};

const MAX_CLEANUP: usize = 169;

pub fn cleanup_expired(db: &mut DB, expiries: &mut Expiries) {
    let rn = Instant::now();
    let mut cleaned: usize = 0;

    while let Some((Reverse(expiry), key)) = expiries.peek().cloned() {
        if cleaned > MAX_CLEANUP || expiry > rn {
            break;
        }

        expiries.pop();

        if let Some(Entry {
            expiry: Some(actual_expiry),
            ..
        }) = db.get(&key)
        {
            if *actual_expiry == expiry {
                db.remove(&key);
            }
        }

        cleaned += 1;
    }
}
