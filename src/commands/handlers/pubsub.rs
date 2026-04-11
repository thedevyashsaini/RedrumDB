use crate::command_handler;
use crate::commands::{Action, Context};
use std::io::Write;

command_handler!(subscribe, args, ctx, {
    let channel = args.get(0).ok_or(b"-ERR missing key".to_vec())?;

    ctx.subscriptions.push(channel.to_vec());
    *ctx.is_pubsub = true;
    ctx.pubsub
        .entry(channel.to_vec())
        .or_insert_with(Vec::new)
        .push(ctx.token);

    let mut res = Vec::with_capacity(23 + channel.len() + 1 + 20 + 2);
    res.extend_from_slice(b"*3\r\n$9\r\nsubscribe\r\n");
    write!(res, "${}\r\n", channel.len()).unwrap();
    res.extend_from_slice(channel);
    write!(res, "\r\n:{}\r\n", ctx.subscriptions.len()).unwrap();
    Ok(res)
});

command_handler!(publish, args, ctx, {
    let channel = args.get(0).ok_or(b"-ERR missing key".to_vec())?;
    let message = args.get(1).ok_or(b"-ERR missing key".to_vec())?;

    let len: usize;
    if let Some(entry) = ctx.pubsub.get(&channel.to_vec()) {
        len = entry.len();
    } else {
        len = 0;
    }

    ctx.actions.push(Action::Publish {
        channel: channel.to_vec(),
        message: message.to_vec(),
    });

    let mut res = Vec::with_capacity(32);
    write!(res, ":{}\r\n", len).unwrap();
    Ok(res)
});

command_handler!(unsubscribe, args, ctx, {
    if *ctx.is_pubsub {
        let channel = args.get(0).ok_or(b"-ERR missing key".to_vec())?;
        let channel_key = channel.to_vec();
        if let Some(index) = ctx.subscriptions.iter().position(|n| n == &channel_key) {
            ctx.subscriptions.remove(index);
            ctx.pubsub.entry(channel_key).and_modify(|x| {
                if let Some(idx) = x.iter().position(|x1| x1.eq(&ctx.token)) {
                    x.remove(idx);
                }
            });
        }
        let mut res = Vec::with_capacity(26 + channel.len() + 1 + 20 + 2);
        res.extend_from_slice(b"*3\r\n$11\r\nunsubscribe\r\n");
        write!(res, "${}\r\n", channel.len()).unwrap();
        res.extend_from_slice(channel);
        write!(res, "\r\n:{}\r\n", ctx.subscriptions.len()).unwrap();
        Ok(res)
    } else {
        Ok(b"-Err not in subscribed mode".to_vec())
    }
});
