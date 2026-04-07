use std::collections::VecDeque;
use std::io::Write;
use std::sync::Arc;

use crate::commands::{Context};
use crate::types::{Entry, Key, Value};
use crate::command_handler;

command_handler!(rpush, args, ctx, {

    let key = args.get(0).ok_or(b"-ERR missing key".to_vec())?;

    if args.len() < 2 {
        return Err(b"-ERR wrong number of arguments\r\n".to_vec());
    }

    let values = &args[1..];

    let list_len: usize;

    let key_bytes: &[u8] = key;

    if let Some(Entry {
        value: Value::List(ref mut list),
        ..
    }) = ctx.db.get_mut(key_bytes)
    {
        for v in values {
            list.push_back(v.to_vec());
        }
        list_len = list.len();
    } else {
        let mut newlist = VecDeque::new();
        for v in values {
            newlist.push_back(v.to_vec());
        }

        list_len = newlist.len();

        let key: Key = Arc::from(*key);

        ctx.db.insert(
            key.clone(),
            Entry {
                value: Value::List(newlist),
                expiry: None,
            },
        );
    }

    let mut res = Vec::with_capacity(32);
    write!(res, ":{}\r\n", list_len).unwrap();
    Ok(res)
});

command_handler!(lpush, args, ctx, {

    let key = args.get(0).ok_or(b"-ERR missing key".to_vec())?;

    if args.len() < 2 {
        return Err(b"-ERR wrong number of arguments\r\n".to_vec());
    }

    let values = &args[1..];

    let list_len: usize;

    let key_bytes: &[u8] = key;

    if let Some(Entry {
        value: Value::List(ref mut list),
        ..
    }) = ctx.db.get_mut(key_bytes)
    {
        for v in values {
            list.push_front(v.to_vec());
        }
        list_len = list.len();
    } else {
        let mut newlist = VecDeque::new();
        for v in values {
            newlist.push_front(v.to_vec());
        }

        list_len = newlist.len();

        let key: Key = Arc::from(*key);

        ctx.db.insert(
            key.clone(),
            Entry {
                value: Value::List(newlist),
                expiry: None,
            },
        );
    }

    let mut res = Vec::with_capacity(32);
    write!(res, ":{}\r\n", list_len).unwrap();
    Ok(res)
});

command_handler!(lrange, args, ctx, {

    let key = args.get(0).ok_or(b"-ERR missing key".to_vec())?;
    let start = args.get(1).ok_or(b"-ERR missing start".to_vec())?;
    let stop = args.get(2).ok_or(b"-ERR missing stop".to_vec())?;

    if let Some(entry) = ctx.db.get(*key) {
        match &entry.value {
            Value::List(list) => {
                let start = std::str::from_utf8(start)
                    .unwrap()
                    .parse::<isize>()
                    .unwrap();
                let stop = std::str::from_utf8(stop).unwrap().parse::<isize>().unwrap();

                let len = list.len() as isize;

                let mut start = if start < 0 { len + start } else { start };
                let mut stop = if stop < 0 { len + stop } else { stop };

                if start < 0 {
                    start = 0;
                }
                if stop >= len {
                    stop = len - 1;
                }

                if start > stop || start >= len {
                    return Ok(b"*0\r\n".to_vec());
                }

                let start = start as usize;
                let count = stop as usize - start + 1;

                let mut res = Vec::with_capacity(count * 16);

                write!(res, "*{}\r\n", count).unwrap();

                for item in list.iter().skip(start).take(count) {
                    write!(res, "${}\r\n", item.len()).unwrap();
                    res.extend_from_slice(item);
                    res.extend_from_slice(b"\r\n");
                }

                Ok(res)
            }
            _ => Err(
                b"-WRONGTYPE Operation against a key holding the wrong kind of value\r\n".to_vec(),
            ),
        }
    } else {
        Ok(b"*0\r\n".to_vec())
    }
});

command_handler!(llen, args, ctx, {

    let key = args.get(0).ok_or(b"-ERR missing key".to_vec())?;

    if let Some(entry) = ctx.db.get(*key) {
        match &entry.value {
            Value::List(list) => {
                let mut res = Vec::with_capacity(32);
                write!(res, ":{}\r\n", list.len()).unwrap();
                Ok(res)
            }
            _ => Err(
                b"-WRONGTYPE Operation against a key holding the wrong kind of value\r\n".to_vec(),
            ),
        }
    } else {
        Ok(b":0\r\n".to_vec())
    }
});

command_handler!(lpop, args, ctx, {

    let key = args.get(0).ok_or(b"-ERR missing key".to_vec())?;

    if let Some(entry) = ctx.db.get_mut(*key) {
        match entry.value {
            Value::List(ref mut list) => {
                if list.len() == 0 {
                    return Ok(b"$-1\r\n".to_vec());
                }

                let count = if let Some(count) = args.get(1) {
                    std::str::from_utf8(count)
                        .unwrap()
                        .parse::<usize>()
                        .unwrap()
                } else {
                    1
                };

                if count == 1 {
                    return if let Some(item) = list.pop_front() {
                        let mut res = Vec::with_capacity(32);
                        write!(res, "${}\r\n", item.len()).unwrap();
                        res.extend_from_slice(&item);
                        res.extend_from_slice(b"\r\n");
                        Ok(res)
                    } else {
                        Ok(b"$-1\r\n".to_vec())
                    };
                }

                let mut res = Vec::new();
                let mut actual = 0;

                for _ in 0..count {
                    if let Some(item) = list.pop_front() {
                        actual += 1;
                        write!(res, "${}\r\n", item.len()).unwrap();
                        res.extend_from_slice(&item);
                        res.extend_from_slice(b"\r\n");
                    } else {
                        break;
                    }
                }

                if actual == 0 {
                    return Ok(b"*0\r\n".to_vec());
                }

                let mut header = Vec::new();
                write!(header, "*{}\r\n", actual).unwrap();

                header.extend_from_slice(&res);

                Ok(header)
            }
            _ => Err(
                b"-WRONGTYPE Operation against a key holding the wrong kind of value\r\n".to_vec(),
            ),
        }
    } else {
        Ok(b"$-1\r\n".to_vec())
    }
});

command_handler!(blpop, args, ctx, {

    let key = args.get(0).ok_or(b"-ERR missing key".to_vec())?;
    let _ = args.get(1).ok_or(b"-ERR missing timeout".to_vec())?;

    if let Some(entry) = ctx.db.get_mut(*key) {
        if let Value::List(ref mut list) = entry.value {
            if let Some(val) = list.pop_front() {
                let mut res = Vec::with_capacity(val.len() + key.len() + 64);

                write!(res, "*2\r\n").unwrap();

                write!(res, "${}\r\n", key.len()).unwrap();
                res.extend_from_slice(key);
                res.extend_from_slice(b"\r\n");

                write!(res, "${}\r\n", val.len()).unwrap();
                res.extend_from_slice(&val);
                res.extend_from_slice(b"\r\n");

                return Ok(res);
            }
        }
    }

    Err(b"__BLOCK__".to_vec())
});
