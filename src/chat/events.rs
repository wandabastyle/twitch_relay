use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::util::time::now_unix_secs;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatEventKind {
    Message,
    Notice,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatEvent {
    pub kind: ChatEventKind,
    pub channel_login: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_login: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_color: Option<String>,
    pub text: String,
    pub parts: Vec<ChatPart>,
    pub sent_at_unix: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChatPart {
    Text {
        text: String,
    },
    Emote {
        id: String,
        code: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        image_url: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ChatChannelStatus {
    pub subscribed: bool,
    pub connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChatChannelRequest {
    pub channel_login: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatSendRequest {
    pub channel_login: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatStatusQuery {
    pub channel_login: String,
}

#[derive(Debug, Deserialize)]
pub struct EmotesQuery {
    pub channel_login: String,
}

#[derive(Debug, Serialize)]
pub struct ChatStatusResponse {
    pub status: ChatChannelStatus,
}

#[derive(Debug, Clone)]
pub(crate) struct EmoteOccurrence {
    pub(crate) id: String,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub fn parse_chat_event(line: &str) -> Option<ChatEvent> {
    let mut rest = line.trim();
    if rest.is_empty() {
        return None;
    }

    let mut tags: HashMap<&str, &str> = HashMap::new();

    if rest.starts_with('@') {
        let (raw_tags, remaining) = rest.split_once(' ')?;
        for pair in raw_tags.trim_start_matches('@').split(';') {
            if let Some((key, value)) = pair.split_once('=') {
                tags.insert(key, value);
            }
        }
        rest = remaining;
    }

    if rest.starts_with(':') {
        let (_, remaining) = rest.split_once(' ')?;
        rest = remaining;
    }

    let (command, tail) = rest.split_once(' ').unwrap_or((rest, ""));
    let trailing = tail.split_once(" :").map(|(_, value)| value).unwrap_or("");

    match command {
        "PRIVMSG" => {
            let mut pieces = tail.split_whitespace();
            let channel = pieces.next()?.trim_start_matches('#').to_ascii_lowercase();
            if channel.is_empty() {
                return None;
            }

            let sender_login = tags.get("login").map(|v| (*v).to_string());
            let sender_display_name = tags.get("display-name").map(|v| (*v).to_string());
            let parts = parse_message_parts(trailing, tags.get("emotes").copied());

            Some(ChatEvent {
                kind: ChatEventKind::Message,
                channel_login: channel,
                sender_login,
                sender_display_name,
                sender_color: resolve_sender_color(
                    tags.get("color").copied(),
                    tags.get("login").copied(),
                ),
                text: trailing.to_string(),
                parts,
                sent_at_unix: now_unix_secs(),
            })
        }
        "NOTICE" => {
            let mut pieces = tail.split_whitespace();
            let channel = pieces.next()?.trim_start_matches('#').to_ascii_lowercase();
            if channel.is_empty() {
                return None;
            }

            Some(ChatEvent {
                kind: ChatEventKind::Notice,
                channel_login: channel,
                sender_login: None,
                sender_display_name: None,
                sender_color: None,
                text: trailing.to_string(),
                parts: vec![ChatPart::Text {
                    text: trailing.to_string(),
                }],
                sent_at_unix: now_unix_secs(),
            })
        }
        _ => None,
    }
}

pub(crate) fn parse_message_parts(message: &str, emotes_tag: Option<&str>) -> Vec<ChatPart> {
    let chars: Vec<char> = message.chars().collect();
    if chars.is_empty() {
        return vec![ChatPart::Text {
            text: String::new(),
        }];
    }

    let mut occurrences = parse_emote_occurrences(emotes_tag, chars.len());
    if occurrences.is_empty() {
        return vec![ChatPart::Text {
            text: message.to_string(),
        }];
    }

    occurrences.sort_by_key(|occurrence| (occurrence.start, occurrence.end));

    let mut parts = Vec::new();
    let mut cursor = 0_usize;

    for occurrence in occurrences {
        if occurrence.start < cursor {
            continue;
        }

        if occurrence.start > cursor {
            let text = chars[cursor..occurrence.start].iter().collect::<String>();
            if !text.is_empty() {
                parts.push(ChatPart::Text { text });
            }
        }

        let emote_text = chars[occurrence.start..=occurrence.end]
            .iter()
            .collect::<String>();

        parts.push(ChatPart::Emote {
            id: occurrence.id,
            code: emote_text,
            image_url: None,
        });
        cursor = occurrence.end.saturating_add(1);
    }

    if cursor < chars.len() {
        let text = chars[cursor..].iter().collect::<String>();
        if !text.is_empty() {
            parts.push(ChatPart::Text { text });
        }
    }

    if parts.is_empty() {
        vec![ChatPart::Text {
            text: message.to_string(),
        }]
    } else {
        parts
    }
}

pub(crate) fn parse_emote_occurrences(emotes_tag: Option<&str>, char_len: usize) -> Vec<EmoteOccurrence> {
    let Some(raw) = emotes_tag else {
        return Vec::new();
    };

    if raw.trim().is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    for emote_def in raw.split('/') {
        let Some((id, positions)) = emote_def.split_once(':') else {
            continue;
        };

        let emote_id = id.trim();
        if emote_id.is_empty() {
            continue;
        }

        for position in positions.split(',') {
            let Some((start_raw, end_raw)) = position.split_once('-') else {
                continue;
            };

            let Ok(start) = start_raw.parse::<usize>() else {
                continue;
            };
            let Ok(end) = end_raw.parse::<usize>() else {
                continue;
            };

            if start > end || end >= char_len {
                continue;
            }

            out.push(EmoteOccurrence {
                id: emote_id.to_string(),
                start,
                end,
            });
        }
    }

    out
}

pub(crate) fn resolve_sender_color(raw_color: Option<&str>, raw_login: Option<&str>) -> Option<String> {
    if let Some(color) = raw_color.and_then(normalize_hex_color) {
        return Some(color);
    }

    let login = raw_login?.trim();
    if login.is_empty() {
        return None;
    }

    Some(fallback_sender_color(login))
}

pub(crate) fn normalize_hex_color(input: &str) -> Option<String> {
    let value = input.trim();
    if value.len() != 7 || !value.starts_with('#') {
        return None;
    }

    if !value.as_bytes()[1..]
        .iter()
        .all(|byte| byte.is_ascii_hexdigit())
    {
        return None;
    }

    Some(value.to_ascii_uppercase())
}

pub(crate) fn fallback_sender_color(login: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in login.trim().to_ascii_lowercase().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    let hue = (hash % 360) as f64;
    let saturation = 0.62;
    let lightness = 0.66;
    let (r, g, b) = hsl_to_rgb(hue, saturation, lightness);
    format!("#{r:02X}{g:02X}{b:02X}")
}

pub(crate) fn hsl_to_rgb(hue_deg: f64, saturation: f64, lightness: f64) -> (u8, u8, u8) {
    let hue = (hue_deg / 360.0).rem_euclid(1.0);
    if saturation <= 0.0 {
        let gray = (lightness.clamp(0.0, 1.0) * 255.0).round() as u8;
        return (gray, gray, gray);
    }

    let q = if lightness < 0.5 {
        lightness * (1.0 + saturation)
    } else {
        lightness + saturation - lightness * saturation
    };
    let p = 2.0 * lightness - q;

    let r = hue_to_channel(p, q, hue + (1.0 / 3.0));
    let g = hue_to_channel(p, q, hue);
    let b = hue_to_channel(p, q, hue - (1.0 / 3.0));
    (r, g, b)
}

pub(crate) fn hue_to_channel(p: f64, q: f64, t: f64) -> u8 {
    let mut value = t;
    if value < 0.0 {
        value += 1.0;
    }
    if value > 1.0 {
        value -= 1.0;
    }

    let channel = if value < 1.0 / 6.0 {
        p + (q - p) * 6.0 * value
    } else if value < 1.0 / 2.0 {
        q
    } else if value < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - value) * 6.0
    } else {
        p
    };

    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}
