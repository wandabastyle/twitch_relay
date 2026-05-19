use std::collections::HashMap;

use serde::{
   Deserialize,
   Serialize,
};

use crate::util::time::now_unix_secs;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatEventKind {
   Message,
   Notice,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatEvent {
   pub kind:                ChatEventKind,
   pub channel_login:       String,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub sender_login:        Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub sender_display_name: Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub sender_color:        Option<String>,
   pub text:                String,
   pub parts:               Vec<ChatPart>,
   pub sent_at_unix:        u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChatPart {
   Text {
      text: String,
   },
   Emote {
      id:        String,
      code:      String,
      #[serde(skip_serializing_if = "Option::is_none")]
      image_url: Option<String>,
   },
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ChatChannelStatus {
   pub subscribed: bool,
   pub connected:  bool,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub error:      Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChatChannelRequest {
   pub channel_login: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatSendRequest {
   pub channel_login: String,
   pub message:       String,
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
pub struct EmoteOccurrence {
   pub(crate) id:    String,
   pub(crate) start: usize,
   pub(crate) end:   usize,
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
   let trailing = tail.split_once(" :").map_or("", |(_, value)| value);

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
      },
      "NOTICE" => {
         let mut pieces = tail.split_whitespace();
         let channel = pieces.next()?.trim_start_matches('#').to_ascii_lowercase();
         if channel.is_empty() {
            return None;
         }

         Some(ChatEvent {
            kind:                ChatEventKind::Notice,
            channel_login:       channel,
            sender_login:        None,
            sender_display_name: None,
            sender_color:        None,
            text:                trailing.to_string(),
            parts:               vec![ChatPart::Text {
               text: trailing.to_string(),
            }],
            sent_at_unix:        now_unix_secs(),
         })
      },
      _ => None,
   }
}

pub fn parse_message_parts(message: &str, emotes_tag: Option<&str>) -> Vec<ChatPart> {
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
         id:        occurrence.id,
         code:      emote_text,
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

pub fn parse_emote_occurrences(emotes_tag: Option<&str>, char_len: usize) -> Vec<EmoteOccurrence> {
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

pub fn resolve_sender_color(raw_color: Option<&str>, raw_login: Option<&str>) -> Option<String> {
   if let Some(color) = raw_color.and_then(normalize_hex_color) {
      return Some(color);
   }

   let login = raw_login?.trim();
   if login.is_empty() {
      return None;
   }

   Some(fallback_sender_color(login))
}

pub fn normalize_hex_color(input: &str) -> Option<String> {
   let value = input.trim();
   if value.len() != 7 || !value.starts_with('#') {
      return None;
   }

   if !value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit) {
      return None;
   }

   Some(value.to_ascii_uppercase())
}

pub fn fallback_sender_color(login: &str) -> String {
   let mut hash: u64 = 0xCBF2_9CE4_8422_2325;
   for byte in login.trim().to_ascii_lowercase().as_bytes() {
      hash ^= u64::from(*byte);
      hash = hash.wrapping_mul(0x0100_0000_01B3);
   }

   let hue = f64::from(u32::try_from(hash % 360).unwrap_or(0));
   let saturation = 0.62;
   let lightness = 0.66;
   let (red, green, blue) = hsl_to_rgb(hue, saturation, lightness);
   format!("#{red:02X}{green:02X}{blue:02X}")
}

pub fn hsl_to_rgb(hue_deg: f64, saturation: f64, lightness: f64) -> (u8, u8, u8) {
   let hue = (hue_deg / 360.0).rem_euclid(1.0);
   if saturation <= 0.0 {
      let gray = float_to_u8(lightness);
      return (gray, gray, gray);
   }

   let hue_q = if lightness < 0.5 {
      lightness * (1.0 + saturation)
   } else {
      lightness.mul_add(-saturation, lightness + saturation)
   };
   let hue_p = 2.0f64.mul_add(lightness, -hue_q);

   let red = hue_to_channel(hue_p, hue_q, hue + (1.0 / 3.0));
   let green = hue_to_channel(hue_p, hue_q, hue);
   let blue = hue_to_channel(hue_p, hue_q, hue - (1.0 / 3.0));
   (red, green, blue)
}

fn float_to_u8(value: f64) -> u8 {
   let clamped = value.clamp(0.0, 1.0);
   let scaled = clamped.mul_add(255.0, 0.5);
   // Values are clamped to [0.0, 1.0], so result is in [0.5, 255.5]
   // Using try_from with unwrap_or is safe since value is clamped
   u8::try_from(scaled.trunc() as i64).unwrap_or(0)
}

pub fn hue_to_channel(p: f64, q: f64, t: f64) -> u8 {
   let mut value = t;
   if value < 0.0 {
      value += 1.0;
   }
   if value > 1.0 {
      value -= 1.0;
   }

   let channel = if value < 1.0 / 6.0 {
      ((q - p) * 6.0).mul_add(value, p)
   } else if value < 1.0 / 2.0 {
      q
   } else if value < 2.0 / 3.0 {
      ((q - p) * (2.0 / 3.0 - value)).mul_add(6.0, p)
   } else {
      p
   };

   float_to_u8(channel)
}
