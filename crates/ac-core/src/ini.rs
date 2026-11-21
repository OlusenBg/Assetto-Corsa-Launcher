//! Lenient, round-trip-preserving INI editor for Assetto Corsa config files.
//!
//! AC's INI files are hand-edited Windows files: CRLF endings, trailing
//! `; comments`, headers with stray spaces (`[GEARUP] `). We keep every line
//! we did not touch byte-for-byte so editing `race.ini` never destroys
//! settings or comments that the launcher does not know about.

/// One physical line of the file.
#[derive(Debug, Clone)]
enum Line {
    /// Blank line, pure comment, or anything we do not understand. Kept verbatim.
    Other(String),
    Section {
        name: String,
        raw: String,
    },
    Entry {
        key: String,
        value: String,
        /// Original text; `None` once the value was modified (then we regenerate).
        raw: Option<String>,
        /// Trailing `; comment` (without the `;`), kept when regenerating.
        comment: Option<String>,
    },
}

/// A parsed INI document.
#[derive(Debug, Clone, Default)]
pub struct Ini {
    lines: Vec<Line>,
}

impl Ini {
    /// Parse INI text. Never fails: unknown lines are preserved as-is.
    pub fn parse(text: &str) -> Self {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let lines = text
            .lines() // handles both \n and \r\n
            .map(|raw| {
                let t = raw.trim();
                if t.starts_with('[') {
                    if let Some(end) = t.find(']') {
                        return Line::Section {
                            name: t[1..end].trim().to_string(),
                            raw: raw.to_string(),
                        };
                    }
                }
                if t.starts_with(';') || t.starts_with('#') || t.is_empty() {
                    return Line::Other(raw.to_string());
                }
                if let Some(eq) = t.find('=') {
                    let key = t[..eq].trim().to_string();
                    let rest = &t[eq + 1..];
                    let (value, comment) = match rest.find(';') {
                        Some(i) => (rest[..i].trim(), Some(rest[i + 1..].trim().to_string())),
                        None => (rest.trim(), None),
                    };
                    return Line::Entry {
                        key,
                        value: value.to_string(),
                        raw: Some(raw.to_string()),
                        comment,
                    };
                }
                Line::Other(raw.to_string())
            })
            .collect();
        Ini { lines }
    }

    /// Get a value. Section and key are matched case-insensitively.
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        let mut in_section = false;
        for line in &self.lines {
            match line {
                Line::Section { name, .. } => in_section = name.eq_ignore_ascii_case(section),
                Line::Entry { key: k, value, .. } if in_section && k.eq_ignore_ascii_case(key) => {
                    return Some(value.as_str());
                }
                _ => {}
            }
        }
        None
    }

    /// Set a value, creating the section and/or key when missing.
    pub fn set(&mut self, section: &str, key: &str, value: impl ToString) {
        let value = value.to_string();
        let mut in_section = false;
        let mut last_in_section: Option<usize> = None; // index to insert after
        for (i, line) in self.lines.iter_mut().enumerate() {
            match line {
                Line::Section { name, .. } => {
                    in_section = name.eq_ignore_ascii_case(section);
                    if in_section {
                        last_in_section = Some(i);
                    }
                }
                Line::Entry {
                    key: k,
                    value: v,
                    raw,
                    ..
                } if in_section => {
                    last_in_section = Some(i);
                    if k.eq_ignore_ascii_case(key) {
                        if *v != value {
                            *v = value;
                            *raw = None;
                        }
                        return;
                    }
                }
                _ => {}
            }
        }
        let entry = Line::Entry {
            key: key.to_string(),
            value,
            raw: None,
            comment: None,
        };
        match last_in_section {
            Some(i) => self.lines.insert(i + 1, entry),
            None => {
                if !matches!(self.lines.last(), None | Some(Line::Other(_))) {
                    self.lines.push(Line::Other(String::new()));
                }
                self.lines.push(Line::Section {
                    name: section.to_string(),
                    raw: format!("[{section}]"),
                });
                self.lines.push(entry);
            }
        }
    }

    /// Remove a whole section (header and entries up to the next header).
    pub fn remove_section(&mut self, section: &str) {
        let mut out = Vec::with_capacity(self.lines.len());
        let mut skipping = false;
        for line in self.lines.drain(..) {
            if let Line::Section { name, .. } = &line {
                skipping = name.eq_ignore_ascii_case(section);
            }
            if !skipping {
                out.push(line);
            }
        }
        self.lines = out;
    }

    /// Remove one key from a section (no-op when absent).
    pub fn remove_key(&mut self, section: &str, key: &str) {
        let mut in_section = false;
        self.lines.retain(|line| match line {
            Line::Section { name, .. } => {
                in_section = name.eq_ignore_ascii_case(section);
                true
            }
            Line::Entry { key: k, .. } if in_section && k.eq_ignore_ascii_case(key) => false,
            _ => true,
        });
    }

    /// Names of all sections, in file order.
    pub fn sections(&self) -> Vec<&str> {
        self.lines
            .iter()
            .filter_map(|l| match l {
                Line::Section { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Serialise with CRLF line endings (what the game's own files use).
    pub fn to_string_crlf(&self) -> String {
        let mut out = String::new();
        for line in &self.lines {
            match line {
                Line::Other(s) => out.push_str(s),
                Line::Section { raw, .. } => out.push_str(raw),
                Line::Entry {
                    key,
                    value,
                    raw,
                    comment,
                } => match raw {
                    Some(r) => out.push_str(r),
                    None => {
                        out.push_str(key);
                        out.push('=');
                        out.push_str(value);
                        if let Some(c) = comment {
                            out.push_str(" ; ");
                            out.push_str(c);
                        }
                    }
                },
            }
            out.push_str("\r\n");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[RACE]\r\nTRACK=magione\r\nAI_LEVEL=98 ; skill\r\n\r\n[LIGHTING] \r\nSUN_ANGLE=-48\r\n";

    #[test]
    fn roundtrip_is_lossless() {
        assert_eq!(Ini::parse(SAMPLE).to_string_crlf(), SAMPLE);
    }

    #[test]
    fn trims_headers_and_strips_comments() {
        let ini = Ini::parse(SAMPLE);
        assert_eq!(ini.get("lighting", "sun_angle"), Some("-48"));
        assert_eq!(ini.get("RACE", "AI_LEVEL"), Some("98"));
    }

    #[test]
    fn set_existing_new_key_and_new_section() {
        let mut ini = Ini::parse(SAMPLE);
        ini.set("RACE", "TRACK", "monza");
        ini.set("RACE", "CARS", 1);
        ini.set("SESSION_0", "TYPE", 1);
        let out = ini.to_string_crlf();
        assert!(out.contains("TRACK=monza\r\n"));
        assert!(out.contains("CARS=1\r\n"));
        assert_eq!(ini.get("SESSION_0", "TYPE"), Some("1"));
        // untouched lines keep their comments
        assert!(out.contains("AI_LEVEL=98 ; skill\r\n"));
    }

    #[test]
    fn remove_section_works() {
        let mut ini = Ini::parse(SAMPLE);
        ini.remove_section("LIGHTING");
        assert!(ini.get("LIGHTING", "SUN_ANGLE").is_none());
        assert_eq!(ini.get("RACE", "TRACK"), Some("magione"));
    }
}
