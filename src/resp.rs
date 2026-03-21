pub fn read_line(buf: &[u8], start: usize) -> Option<(String, usize)> {
    for i in start..buf.len() - 1 {
        if buf[i] == b'\r' && buf[i + 1] == b'\n' {
            let line: String = std::str::from_utf8(&buf[start..i]).ok()?.to_string();
            return Some((line, i + 2));
        }
    }
    None
}

pub fn parse_bulk_string(buf: &[u8], start: usize) -> Option<(String, usize)> {
    if buf.get(start)? != &b'$' {
        return None;
    }

    let (len_str, mut pos) = read_line(buf, start + 1)?;
    let len: usize = len_str.parse().ok()?;

    let end = pos + len;
    if end + 2 > buf.len() {
        return None;
    }

    let data: String = std::str::from_utf8(&buf[pos..end]).ok()?.to_string();
    pos = end + 2;

    Some((data, pos))
}

pub fn parse_command(buf: &[u8]) -> Option<(Vec<String>, usize)> {

    if buf.get(0)? != &b'*' {
        return None;
    }

    let (count_str, mut pos) = read_line(buf, 1)?;
    let count: usize = count_str.parse().ok()?;

    let mut args: Vec<String> = Vec::with_capacity(count);

    for _ in 0..count {
        let (arg, new_pos) = parse_bulk_string(buf, pos)?;
        args.push(arg);
        pos = new_pos;
    }

    Some((args, pos))
}