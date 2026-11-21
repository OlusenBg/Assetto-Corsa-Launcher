//! Parser for AC's `ui_car.json` / `ui_track.json` / `ui_skin.json`.
//!
//! These files are "almost JSON". Observed deviations in the shipped content:
//! raw newlines/tabs inside strings, non-UTF-8 bytes (Latin-1 degree signs),
//! and (in mods) trailing commas and stray backslashes. We sanitise the text
//! and hand it to `serde_json`.

use serde_json::Value;

/// Parse possibly-malformed AC JSON. Returns `None` if it is beyond repair.
pub fn parse(bytes: &[u8]) -> Option<Value> {
    let text = decode(bytes);
    serde_json::from_str(&sanitize(&text)).ok()
}

/// UTF-8 if valid, otherwise Latin-1 (every byte maps to the same code point).
fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

fn sanitize(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut in_string = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            match c {
                '\\' => match chars.get(i + 1) {
                    Some(&n) if "\"\\/bfnrtu".contains(n) => {
                        out.push('\\');
                        out.push(n);
                        i += 1;
                    }
                    // Invalid escape such as "\ " or a Windows path: keep the backslash literally.
                    _ => out.push_str("\\\\"),
                },
                '"' => {
                    in_string = false;
                    out.push('"');
                }
                '\n' => out.push_str("\\n"),
                '\r' => {}
                '\t' => out.push_str("\\t"),
                c if (c as u32) < 0x20 => out.push(' '),
                c => out.push(c),
            }
        } else {
            match c {
                '"' => {
                    in_string = true;
                    out.push('"');
                }
                // Drop trailing commas before ] or }.
                ',' => {
                    let next = chars[i + 1..].iter().find(|c| !c.is_whitespace());
                    if !matches!(next, Some(']') | Some('}')) {
                        out.push(',');
                    }
                }
                c => out.push(c),
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_newlines_in_strings() {
        let v = parse(b"{\"d\": \"a\nb\tc\",}").unwrap();
        assert_eq!(v["d"], "a\nb\tc");
    }

    #[test]
    fn latin1_bytes() {
        let v = parse(b"{\"g\": [\"50.3\xB0 N\"]}").unwrap();
        assert_eq!(v["g"][0], "50.3\u{b0} N");
    }

    #[test]
    fn trailing_commas_and_bad_escapes() {
        let v = parse(br#"{"a": [1,2,], "p": "C:\x"}"#).unwrap();
        assert_eq!(v["a"][1], 2);
        assert_eq!(v["p"], "C:\\x");
    }

    #[test]
    fn garbage_is_none() {
        assert!(parse(b"not json").is_none());
    }
}
