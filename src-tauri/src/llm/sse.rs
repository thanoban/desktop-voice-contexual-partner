//! Byte-oriented Server-Sent Events decoder shared by remote chat adapters.
//!
//! Frames are accumulated as raw bytes and only decoded once a whole line is
//! available, so UTF-8 sequences and JSON payloads may be split arbitrarily
//! across network chunks. Lines end with `\n` or `\r\n`; a blank line dispatches
//! the pending event. Comment lines (`:`) and `id`/`retry` fields are ignored.

use anyhow::{anyhow, Result};

const MAX_SSE_EVENT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
}

#[derive(Default)]
pub struct SseDecoder {
    buffer: Vec<u8>,
    event: Option<String>,
    data: Option<String>,
}

impl SseDecoder {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<SseEvent>> {
        self.buffer.extend_from_slice(bytes);
        if self.buffer.len() > MAX_SSE_EVENT_BYTES && !self.buffer.contains(&b'\n') {
            return Err(anyhow!("Provider stream frame exceeded the size limit"));
        }
        let mut events = Vec::new();
        while let Some(newline) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.buffer.drain(..=newline).collect();
            if let Some(event) = self.process_line(trim_line_end(&line))? {
                events.push(event);
            }
        }
        Ok(events)
    }

    /// Flush a final unterminated line and any event missing its blank line.
    pub fn finish(&mut self) -> Result<Vec<SseEvent>> {
        let mut events = Vec::new();
        let line = std::mem::take(&mut self.buffer);
        if !line.is_empty() {
            if let Some(event) = self.process_line(trim_line_end(&line))? {
                events.push(event);
            }
        }
        if let Some(event) = self.dispatch() {
            events.push(event);
        }
        Ok(events)
    }

    fn process_line(&mut self, line: &[u8]) -> Result<Option<SseEvent>> {
        if line.is_empty() {
            return Ok(self.dispatch());
        }
        let line = std::str::from_utf8(line)
            .map_err(|_| anyhow!("Provider stream contained invalid UTF-8"))?;
        if line.starts_with(':') {
            return Ok(None);
        }
        let (field, value) = match line.split_once(':') {
            Some((field, value)) => (field, value.strip_prefix(' ').unwrap_or(value)),
            None => (line, ""),
        };
        match field {
            "event" => self.event = Some(value.to_string()),
            "data" => {
                // Multiple data lines in one event are joined with a newline.
                match self.data.as_mut() {
                    Some(data) => {
                        data.push('\n');
                        data.push_str(value);
                    }
                    None => self.data = Some(value.to_string()),
                }
                if self.data.as_ref().map_or(0, String::len) > MAX_SSE_EVENT_BYTES {
                    return Err(anyhow!("Provider stream event exceeded the size limit"));
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn dispatch(&mut self) -> Option<SseEvent> {
        let event = self.event.take();
        self.data.take().map(|data| SseEvent { event, data })
    }
}

fn trim_line_end(line: &[u8]) -> &[u8] {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line.strip_suffix(b"\r").unwrap_or(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_handles_split_utf8_and_json() {
        let payload = "event: delta\ndata: {\"text\":\"hi 🌍\"}\n\n";
        let bytes = payload.as_bytes();
        let split = payload.find('🌍').unwrap() + 2;
        let mut decoder = SseDecoder::default();
        assert!(decoder.push(&bytes[..split]).unwrap().is_empty());
        let events = decoder.push(&bytes[split..]).unwrap();
        assert_eq!(
            events,
            vec![SseEvent {
                event: Some("delta".into()),
                data: "{\"text\":\"hi 🌍\"}".into()
            }]
        );
    }

    #[test]
    fn decoder_reads_many_events_crlf_comments_and_multiline_data() {
        let mut decoder = SseDecoder::default();
        let events = decoder
            .push(b": keep-alive\r\ndata: a\r\n\r\nid: 7\ndata: b\ndata: c\n\ndata: [DONE]\n\n")
            .unwrap();
        let data: Vec<_> = events.iter().map(|event| event.data.as_str()).collect();
        assert_eq!(data, vec!["a", "b\nc", "[DONE]"]);
        assert!(events.iter().all(|event| event.event.is_none()));
    }

    #[test]
    fn decoder_flushes_unterminated_final_event() {
        let mut decoder = SseDecoder::default();
        assert!(decoder.push(b"data: tail").unwrap().is_empty());
        assert_eq!(decoder.finish().unwrap()[0].data, "tail");
    }

    #[test]
    fn decoder_ignores_events_without_data() {
        let mut decoder = SseDecoder::default();
        assert!(decoder.push(b"event: ping\n\n").unwrap().is_empty());
    }

    #[test]
    fn decoder_rejects_oversized_and_invalid_frames() {
        let mut decoder = SseDecoder::default();
        assert!(decoder.push(&vec![b'a'; MAX_SSE_EVENT_BYTES + 1]).is_err());
        let mut decoder = SseDecoder::default();
        assert!(decoder.push(b"data: \xff\xfe\n").is_err());
    }
}
