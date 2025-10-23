use std::io::{self, Read};
use std::path::PathBuf;

use crate::error::{EditError, EditResult};

#[derive(Debug, Clone)]
pub enum PayloadSource {
    File(PathBuf),
    Stdin,
    Inline(String),
}

pub fn load_payload(source: PayloadSource) -> EditResult<String> {
    match source {
        PayloadSource::File(path) => {
            let content = std::fs::read_to_string(&path).map_err(|err| {
                EditError::InvalidContent(format!(
                    "failed to read payload file '{}': {err}",
                    path.display()
                ))
            })?;
            Ok(content)
        }
        PayloadSource::Stdin => {
            let mut buffer = String::new();
            let mut handle = io::stdin();
            handle.read_to_string(&mut buffer).map_err(|err| {
                EditError::InvalidContent(format!("failed to read stdin payload: {err}"))
            })?;
            Ok(buffer)
        }
        PayloadSource::Inline(raw) => parse_inline(&raw),
    }
}

fn parse_inline(raw: &str) -> EditResult<String> {
    let mut chars = raw.chars();
    let mut output = String::with_capacity(raw.len());

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }

        let Some(next) = chars.next() else {
            return Err(EditError::InvalidContent(
                "unterminated escape sequence".to_string(),
            ));
        };

        match next {
            'n' => output.push('\n'),
            't' => output.push('\t'),
            '\\' => output.push('\\'),
            '"' => output.push('"'),
            _ => {
                return Err(EditError::InvalidContent(format!(
                    "unsupported escape sequence: \\{next}"
                )))
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_inline_payload() {
        let parsed = parse_inline("Hello\\nWorld\\t\\\\\"").unwrap();
        assert_eq!(parsed, "Hello\nWorld\t\\\"");
    }

    #[test]
    fn rejects_unknown_escape() {
        let err = parse_inline("Hello\\rWorld").unwrap_err();
        assert!(matches!(err, EditError::InvalidContent(_)));
    }
}
