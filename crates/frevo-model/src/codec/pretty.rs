//! The indented form: the stored form with whitespace between its tokens.
//!
//! Task 23 of `add-fhirpath-and-codegen-r4` (`model.canonical.006`). The
//! indented form is the stored form -- generated element order -- laid out
//! for a reader, two spaces per level and one member or item per line, which
//! is the layout HAPI's own pretty printer uses. It is produced from the
//! compact stored form by inserting whitespace between tokens and nothing
//! else: every token, including every string and every number, is copied
//! byte for byte, so a decimal keeps its presented form here too and the two
//! forms can differ only in whitespace.

/// Append `compact`, a compact JSON document, to `out` with indentation.
///
/// The input is the codec's own compact output, so it carries no whitespace
/// outside strings; any that it did carry would be copied rather than
/// dropped, so a caller handing in something else gets valid JSON either way.
pub fn indent_into(compact: &[u8], out: &mut Vec<u8>) {
    out.reserve(compact.len() + compact.len() / 2);
    let mut depth = 0usize;
    let mut at = 0usize;
    while at < compact.len() {
        let byte = compact[at];
        match byte {
            b'"' => {
                let end = string_end(compact, at);
                out.extend_from_slice(&compact[at..end]);
                at = end;
                continue;
            }
            b'{' | b'[' => {
                let close = if byte == b'{' { b'}' } else { b']' };
                out.push(byte);
                if compact.get(at + 1) == Some(&close) {
                    // An empty object or array stays on one line.
                    out.push(close);
                    at += 2;
                    continue;
                }
                depth += 1;
                newline(depth, out);
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                newline(depth, out);
                out.push(byte);
            }
            b',' => {
                out.push(byte);
                newline(depth, out);
            }
            b':' => out.extend_from_slice(b": "),
            other => out.push(other),
        }
        at += 1;
    }
}

/// The indented form of `compact`, as a new buffer.
pub fn indent(compact: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    indent_into(compact, &mut out);
    out
}

fn newline(depth: usize, out: &mut Vec<u8>) {
    out.push(b'\n');
    for _ in 0..depth {
        out.extend_from_slice(b"  ");
    }
}

/// The offset one past the closing quote of the string opening at `start`.
fn string_end(bytes: &[u8], start: usize) -> usize {
    let mut at = start + 1;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' => at += 2,
            b'"' => return at + 1,
            _ => at += 1,
        }
    }
    bytes.len()
}

#[cfg(test)]
mod tests {
    use super::indent;

    /// Remove the whitespace outside strings.
    fn strip(text: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut in_string = false;
        let mut escaped = false;
        for &byte in text {
            if in_string {
                out.push(byte);
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    in_string = false;
                }
            } else if byte == b'"' {
                in_string = true;
                out.push(byte);
            } else if !matches!(byte, b' ' | b'\n') {
                out.push(byte);
            }
        }
        out
    }

    // covers: model.canonical.006
    #[test]
    fn indentation_inserts_whitespace_between_tokens_and_nothing_else() {
        let compact = br#"{"a":[1.50,{}],"b":"x, \"y\": {z}","c":[]}"#;
        let pretty = indent(compact);
        assert_eq!(
            String::from_utf8_lossy(&pretty),
            "{\n  \"a\": [\n    1.50,\n    {}\n  ],\n  \"b\": \"x, \\\"y\\\": {z}\",\n  \"c\": []\n}"
        );
        assert_eq!(strip(&pretty), compact.to_vec());
    }
}
