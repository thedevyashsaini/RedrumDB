use crate::command_handler;
use crate::commands::Context;
use crate::types::{Entry, Key, Value};
use std::cmp::Reverse;
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

command_handler!(set, args, ctx, {
    let key = args.get(0).ok_or(b"-ERR missing key".to_vec())?;
    let value = args.get(1).ok_or(b"-ERR missing value".to_vec())?;

    let key: Key = Arc::from(*key);

    let mut expiry: Option<Instant> = None;

    let rn = Instant::now();
    if args.len() > 2 {
        let option = std::str::from_utf8(args[2]).unwrap().to_uppercase();

        if option == "EX" || option == "PX" {
            let exp = std::str::from_utf8(args.get(3).ok_or(b"-ERR missing EX value".to_vec())?)
                .unwrap()
                .parse::<u64>()
                .map_err(|_| b"-ERR invalid EX/PX value".to_vec())?;

            let duration = if option == "PX" {
                std::time::Duration::from_millis(exp)
            } else {
                std::time::Duration::from_secs(exp)
            };

            expiry = Some(rn + duration);
        }
    }

    if let Some(exp) = expiry {
        ctx.expiries.push((Reverse(exp), key.clone()));
    }

    ctx.db.insert(
        key.clone(),
        Entry {
            value: Value::String(value.to_vec()),
            expiry,
        },
    );

    Ok(b"+OK\r\n".to_vec())
});

command_handler!(get, args, ctx, {
    let key = args.get(0).ok_or(b"-ERR missing key".to_vec())?;

    if let Some(entry) = ctx.db.get(*key) {
        if let Some(exp) = entry.expiry {
            if Instant::now() >= exp {
                ctx.db.remove(*key);
                return Ok(b"$-1\r\n".to_vec());
            }
        }

        match &entry.value {
            Value::String(val) => {
                let mut res = Vec::with_capacity(val.len() + 32);
                write!(res, "${}\r\n", val.len()).unwrap();
                res.extend_from_slice(val);
                res.extend_from_slice(b"\r\n");
                Ok(res)
            }
            _ => Err(
                b"-WRONGTYPE Operation against a key holding the wrong kind of value\r\n".to_vec(),
            ),
        }
    } else {
        Ok(b"$-1\r\n".to_vec())
    }
});
