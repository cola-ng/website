//! Bounded SSE framing, incremental JSON string decoding and spoken sentences.
pub struct Sse { pending:Vec<u8>, max_event:usize }
impl Default for Sse {fn default()->Self {Self{pending:Vec::new(),max_event:128*1024}}}
impl Sse {
    pub fn audio()->Self {Self{pending:Vec::new(),max_event:512*1024}}
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>, String> {
        self.pending.extend_from_slice(bytes);
        let mut events = Vec::new();
        loop {
            let split = self
                .pending
                .windows(2)
                .position(|b| b == b"\n\n")
                .map(|at| (at, 2))
                .into_iter()
                .chain(
                    self.pending
                        .windows(4)
                        .position(|b| b == b"\r\n\r\n")
                        .map(|at| (at, 4)),
                )
                .min_by_key(|(at, _)| *at);
            let Some((at, size)) = split else { break };
            if at>self.max_event {return Err("SSE event exceeds limit".into());}
            let frame = String::from_utf8(self.pending.drain(..at + size).collect())
                .map_err(|_| "invalid SSE UTF-8")?;
            let data = frame
                .lines()
                .filter_map(|line| {
                    line.strip_prefix("data:")
                        .map(|s| s.strip_prefix(' ').unwrap_or(s))
                })
                .collect::<Vec<_>>()
                .join("\n");
            if !data.is_empty() {
                events.push(data);
            }
        }
        if self.pending.len()>self.max_event {return Err("SSE event exceeds limit".into());}
        Ok(events)
    }
}

/// The prompt requires reply_en first. Only decode that root string; never
/// send analysis, reasoning channels, or JSON syntax to speech synthesis.
pub fn reply_prefix(raw: &str) -> Result<Option<(String, bool)>, String> {
    let mut text = raw.trim_start();
    if text.starts_with("```") {
        let Some((_, rest)) = text.split_once('\n') else {
            return Ok(None);
        };
        text = rest.trim_start();
    }
    let Some(rest) = text.strip_prefix('{') else {
        return Ok(None);
    };
    let rest = rest.trim_start();
    let Some(rest) = rest.strip_prefix("\"reply_en\"") else {
        return Ok(None);
    };
    let Some(rest) = rest.trim_start().strip_prefix(':') else {
        return Ok(None);
    };
    let Some(rest) = rest.trim_start().strip_prefix('"') else {
        return Ok(None);
    };
    let bytes = rest.as_bytes();
    let mut at = 0;
    let mut out = String::new();
    while at < bytes.len() {
        match bytes[at] {
            b'"' => return Ok(Some((out, true))),
            b'\\' => {
                if at + 1 >= bytes.len() {
                    break;
                }
                let escaped = bytes[at + 1];
                if escaped == b'u' {
                    if at + 6 > bytes.len() {
                        break;
                    }
                    let hex = |s: &[u8]| {
                        std::str::from_utf8(s)
                            .ok()
                            .and_then(|s| u16::from_str_radix(s, 16).ok())
                            .ok_or_else(|| "invalid JSON unicode".to_owned())
                    };
                    let first = hex(&bytes[at + 2..at + 6])?;
                    let (code, size) = if (0xd800..=0xdbff).contains(&first) {
                        if at + 12 > bytes.len() {
                            break;
                        }
                        if &bytes[at + 6..at + 8] != b"\\u" {
                            return Err("invalid JSON surrogate pair".into());
                        }
                        let low = hex(&bytes[at + 8..at + 12])?;
                        if !(0xdc00..=0xdfff).contains(&low) {
                            return Err("invalid JSON surrogate pair".into());
                        }
                        (
                            0x10000
                                + ((u32::from(first) - 0xd800) << 10)
                                + (u32::from(low) - 0xdc00),
                            12,
                        )
                    } else {
                        (u32::from(first), 6)
                    };
                    out.push(char::from_u32(code).ok_or("invalid JSON code point")?);
                    at += size;
                } else {
                    out.push(match escaped {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        _ => return Err("invalid JSON escape".into()),
                    });
                    at += 2;
                }
            }
            0..=31 => return Err("unescaped JSON control character".into()),
            _ => {
                let c = rest[at..].chars().next().unwrap();
                out.push(c);
                at += c.len_utf8();
            }
        }
    }
    Ok(Some((out, false)))
}

#[derive(Default)]
pub struct Sentences {
    pending: String,
}
impl Sentences {
    pub fn push(&mut self, delta: &str) -> Vec<String> {
        self.pending.push_str(delta);
        let mut out = Vec::new();
        loop {
            let mut split = None;
            for (at, c) in self.pending.char_indices() {
                if matches!(c, '.' | '?' | '!' | ';') && at >= 8 {
                    let rest = &self.pending[at + c.len_utf8()..];
                    if rest.starts_with(char::is_whitespace) {
                        let word = self.pending[..at]
                            .split_whitespace()
                            .last()
                            .unwrap_or("")
                            .to_lowercase();
                        if c != '.'
                            || !["mr", "mrs", "ms", "dr", "e.g", "i.e", "vs"]
                                .contains(&word.as_str())
                        {
                            split = Some(at + c.len_utf8());
                            break;
                        }
                    }
                }
                if at >= 180 && c.is_whitespace() {
                    split = Some(at);
                    break;
                }
            }
            let Some(at) = split else { break };
            let piece = self.pending.drain(..at).collect::<String>();
            if !piece.trim().is_empty() {
                out.push(piece.trim().to_owned());
            }
        }
        out
    }
    pub fn finish(&mut self) -> Option<String> {
        let text = std::mem::take(&mut self.pending);
        (!text.trim().is_empty()).then(|| text.trim().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sse_handles_utf8_crlf_and_multiline_at_every_byte_boundary() {
        let wire = "data: {\"text\":\"你好\"}\r\n\r\n: ping\n\ndata: [DONE]\n\n";
        let mut parser = Sse::default();
        let mut out = Vec::new();
        for byte in wire.as_bytes() {
            out.extend(parser.push(&[*byte]).unwrap());
        }
        assert_eq!(out, vec!["{\"text\":\"你好\"}", "[DONE]"]);
    }
    #[test]
    fn reply_decoder_never_speaks_metadata_or_partial_surrogates() {
        let raw =
            r#"{"reply_en":"Hello \"friend\". \uD83D\uDE00","issues":[{"original":"private"}]}"#;
        for at in 0..raw.len() {
            let _ = reply_prefix(&raw[..at]).unwrap();
        }
        assert_eq!(
            reply_prefix(raw).unwrap(),
            Some(("Hello \"friend\". 😀".into(), true))
        );
        assert!(
            reply_prefix(r#"{"reply_en":"\uD83D"#)
                .unwrap()
                .unwrap()
                .0
                .is_empty()
        );
        assert!(
            reply_prefix(r#"{"issues":"private","reply_en":"Hello"}"#)
                .unwrap()
                .is_none()
        );
        assert!(reply_prefix(r#"{"reply_en":"\u你abc"}"#).is_err());
    }
    #[test]
    fn mixed_sse_delimiters_do_not_merge_events() {
        assert_eq!(
            Sse::default().push(b"data: a\r\n\r\ndata: b\n\n").unwrap(),
            vec!["a", "b"]
        );
    }
    #[test]
    fn sentences_preserve_abbreviations_decimals_and_unicode() {
        let mut s = Sentences::default();
        let mut out = Vec::new();
        for c in "Ask Dr. Smith about 3.14. Then say hello! Fine.".chars() {
            out.extend(s.push(&c.to_string()));
        }
        out.extend(s.finish());
        assert_eq!(
            out,
            vec!["Ask Dr. Smith about 3.14.", "Then say hello!", "Fine."]
        );
    }
    #[test]
    fn coalesced_audio_events_are_bounded_individually(){
        let frame=format!("data: {}\n\n","a".repeat(90_000));let mut s=Sse::default();assert_eq!(s.push(format!("{frame}{frame}").as_bytes()).unwrap().len(),2);
        assert!(Sse::default().push(&vec![b'x';129*1024]).is_err());assert!(Sse::audio().push(&vec![b'x';513*1024]).is_err());
    }

}
