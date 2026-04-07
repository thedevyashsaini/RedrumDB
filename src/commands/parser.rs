pub fn read_line(buf: &[u8], start: usize) -> Option<(usize, usize)> {
    for i in start..buf.len() - 1 {
        if buf[i] == b'\r' && buf[i + 1] == b'\n' {
            return Some((start, i + 2));
        }
    }
    None
}

pub fn parse_bulk_string(buf: &[u8], start: usize) -> Option<(&[u8], usize)> {
    if buf.get(start)? != &b'$' {
        return None;
    }

    let (len_start, len_end) = read_line(buf, start + 1)?;
    let len = std::str::from_utf8(&buf[len_start..len_end - 2])
        .ok()?
        .parse::<usize>()
        .ok()?;

    let data_start = len_end;
    let data_end = data_start + len;

    if data_end + 2 > buf.len() {
        return None;
    }

    Some((&buf[data_start..data_end], data_end + 2))
}

pub fn normalize_upper<'a>(cmd: &[u8], buf: &'a mut [u8]) -> &'a [u8] {
    for (i, b) in cmd.iter().enumerate() {
        buf[i] = b.to_ascii_uppercase();
    }
    &buf[..cmd.len()]
}

pub struct Command<'a> {
    pub name: &'a [u8],
    pub args: Vec<&'a [u8]>,
}

pub fn parse_command(buf: &[u8]) -> Result<Command<'_>, Vec<u8>> {
    if buf.get(0) != Some(&b'*') {
        return Err(b"No Command".to_vec());
    }

    let (count_start, count_end) = read_line(buf, 1).ok_or("Invalid")?;
    let count = std::str::from_utf8(&buf[count_start..count_end - 2])
        .map_err(|_| "Invalid")?
        .parse::<usize>()
        .map_err(|_| "Invalid")?;

    let mut pos = count_end;

    let (cmd_bytes, new_pos) = parse_bulk_string(buf, pos).ok_or("Invalid")?;
    pos = new_pos;

    let mut args = Vec::with_capacity(count - 1);

    for _ in 0..count - 1 {
        let (arg, new_pos) = parse_bulk_string(buf, pos).ok_or("Invalid")?;
        args.push(arg);
        pos = new_pos;
    }

    Ok(Command {
        name: cmd_bytes,
        args,
    })
}
