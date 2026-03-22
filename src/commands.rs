use std::collections::HashMap;

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
    let len = std::str::from_utf8(&buf[len_start..len_end - 2]).ok()?.parse::<usize>().ok()?;

    let data_start = len_end;
    let data_end = data_start + len;

    if data_end + 2 > buf.len() {
        return None;
    }

    Some((&buf[data_start..data_end], data_end + 2))
}

#[derive(Debug)]
pub enum CommandType {
    // test
    PING,
    ECHO,

    // KV
    SET,
    GET
}

#[derive(Debug)]
pub struct Command<'a> {
    pub cmd_type: CommandType,
    pub args: Vec<&'a [u8]>,
}

impl Command<'_> {
    pub(crate) fn process(&self, db: &mut HashMap<Vec<u8>, Vec<u8>>) -> Result<String, String> {
        match self.cmd_type {
            CommandType::PING => {
                if !self.args.is_empty() {
                    let arg = std::str::from_utf8(self.args[0]).unwrap();
                    Ok(format!("${}\r\n{}\r\n", arg.len(), arg))
                } else {
                    Ok("+PONG\r\n".to_string())
                }
            }
            CommandType::ECHO => {
                if self.args.is_empty() {
                    Err("-ERR wrong number of arguments\r\n".to_string())
                } else {
                    let arg = std::str::from_utf8(self.args[0]).unwrap();
                    Ok(format!("${}\r\n{}\r\n", arg.len(), arg))
                }
            }
            CommandType::SET => {
                let key = self.args.get(0).ok_or("ERR missing key")?;
                let value = self.args.get(1).ok_or("ERR missing value")?;

                db.insert(key.to_vec(), value.to_vec());
                Ok("+OK\r\n".to_string())
            }

            CommandType::GET => {
                let key = self.args.get(0).ok_or("ERR missing key")?;

                if let Some(val) = db.get(*key) {
                    Ok(format!("${}\r\n{}\r\n", val.len(), std::str::from_utf8(val).unwrap()))
                } else {
                    Ok("$-1\r\n".to_string())
                }
            }
        }
    }
}

pub fn parse_command(buf: &[u8]) -> Result<Command<'_>, String> {
    if buf.get(0) != Some(&b'*') {
        return Err("No Command".to_string());
    }

    let (count_start, count_end) = read_line(buf, 1).ok_or("Invalid")?;
    let count = std::str::from_utf8(&buf[count_start..count_end - 2])
        .map_err(|_| "Invalid")?
        .parse::<usize>()
        .map_err(|_| "Invalid")?;

    let mut pos = count_end;

    let (cmd_bytes, new_pos) = parse_bulk_string(buf, pos).ok_or("Invalid")?;
    pos = new_pos;

    let cmd = std::str::from_utf8(cmd_bytes).map_err(|_| "Invalid")?;

    let mut args = Vec::with_capacity(count - 1);

    for _ in 0..count - 1 {
        let (arg, new_pos) = parse_bulk_string(buf, pos).ok_or("Invalid")?;
        args.push(arg);
        pos = new_pos;
    }

    let cmd_type = match cmd.to_uppercase().as_str() {
        "PING" => CommandType::PING,
        "ECHO" => CommandType::ECHO,
        "SET" => CommandType::SET,
        "GET" => CommandType::GET,
        _ => return Err("Invalid command".to_string()),
    };

    Ok(Command { cmd_type, args })
}