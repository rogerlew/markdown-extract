use std::io::{self, BufRead};

#[derive(Debug, Clone)]
pub struct LineRecord {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

pub fn read_lines<R: BufRead>(reader: &mut R) -> io::Result<Vec<LineRecord>> {
    let mut lines = Vec::new();
    let mut buffer = String::new();
    let mut offset = 0usize;

    loop {
        buffer.clear();
        let bytes_read = reader.read_line(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        let mut line = buffer.clone();

        if line.ends_with('\n') {
            line.pop();

            if line.ends_with('\r') {
                line.pop();
            }
        }

        lines.push(LineRecord {
            text: line,
            start: offset,
            end: offset + bytes_read,
        });

        offset += bytes_read;
    }

    Ok(lines)
}
