use std::{
   collections::{
      HashMap,
      HashSet,
   },
   sync::Arc,
   time::Duration,
};

use futures_util::StreamExt;
use serde::{
   Deserialize,
   Serialize,
};
use tokio::sync::{
   RwLock,
   broadcast,
};
use tokio_tungstenite::connect_async;

use crate::twitch_auth::TwitchAuthService;

const EVENTSUB_URL: &str = "wss://eventsub.wss.twitch.tv/ws";
const INITIAL_RECONNECT_DELAY: Duration = Duration::from_secs(2);
const MAX_RECONNECT_DELAY: Duration = Duration::from_mins(1);
const DEFAULT_KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(15);
const KEEPALIVE_TIMEOUT_GRACE: Duration = Duration::from_secs(2);
type RaidGrantKey = (String, String, String);

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RaidEvent {
   pub event_id:                    String,
   pub watch_ticket:                String,
   pub from_broadcaster_user_id:    String,
   pub from_broadcaster_user_login: String,
   pub to_broadcaster_user_id:      String,
   pub to_broadcaster_user_login:   String,
   pub to_broadcaster_user_name:    String,
   pub viewers:                     u64,
}

#[derive(Debug, Clone)]
struct Registration {
   session_token: String,
   channel_login: String,
   sender:        broadcast::Sender<RaidEvent>,
   receivers:     usize,
}

#[derive(Debug, Clone)]
pub struct TwitchEventSubService {
   auth:          TwitchAuthService,
   registrations: Arc<RwLock<HashMap<String, Registration>>>,
   grants:        Arc<RwLock<HashMap<RaidGrantKey, u64>>>,
}

impl TwitchEventSubService {
   pub fn new(auth: TwitchAuthService) -> Self {
      Self {
         auth,
         registrations: Arc::new(RwLock::new(HashMap::new())),
         grants: Arc::new(RwLock::new(HashMap::new())),
      }
   }

   pub async fn subscribe(
      &self,
      ticket: &str,
      session_token: &str,
      channel_login: &str,
   ) -> broadcast::Receiver<RaidEvent> {
      let mut registrations = self.registrations.write().await;
      let normalized_channel = channel_login.to_ascii_lowercase();
      if let Some(existing) = registrations.get(ticket)
         && existing.session_token == session_token
         && existing.channel_login == normalized_channel
      {
         let receiver = existing.sender.subscribe();
         if let Some(existing) = registrations.get_mut(ticket) {
            existing.receivers = existing.receivers.saturating_add(1);
         }
         return receiver;
      }
      let (sender, receiver) = broadcast::channel(8);
      registrations.insert(ticket.to_string(), Registration {
         session_token: session_token.to_string(),
         channel_login: normalized_channel,
         sender:        sender.clone(),
         receivers:     1,
      });
      drop(registrations);

      let service = self.clone();
      let ticket = ticket.to_string();
      tokio::spawn(async move {
         let mut reconnect_delay = INITIAL_RECONNECT_DELAY;
         loop {
            if !service.registration_matches(&ticket, &sender).await {
               break;
            }
            if let Err(error) = service.run_subscription(&ticket, sender.clone()).await {
               tracing::warn!(error = %error, ticket = %ticket, retry_delay_secs = reconnect_delay.as_secs(), "raid EventSub subscription reconnecting");
            }
            tokio::time::sleep(reconnect_delay).await;
            reconnect_delay = next_reconnect_delay(reconnect_delay);
         }
      });
      receiver
   }

   pub async fn unsubscribe(&self, ticket: &str, session_token: &str) {
      let mut registrations = self.registrations.write().await;
      release_registration(&mut registrations, ticket, session_token);
   }

   pub async fn consume_grant(&self, ticket: &str, session_token: &str, destination: &str) -> bool {
      let now = crate::util::time::now_unix_secs();
      let mut grants = self.grants.write().await;
      grants.retain(|_, expires| *expires > now);
      grants
         .remove(&(
            ticket.to_string(),
            session_token.to_string(),
            destination.trim().to_ascii_lowercase(),
         ))
         .is_some()
   }

   async fn run_subscription(
      &self,
      ticket: &str,
      sender: broadcast::Sender<RaidEvent>,
   ) -> Result<(), String> {
      let registration = self
         .registrations
         .read()
         .await
         .get(ticket)
         .cloned()
         .ok_or("watch registration ended")?;
      if !registration.sender.same_channel(&sender) {
         return Err("watch registration was replaced".to_string());
      }
      let identity = self
         .auth
         .fetch_channel_identity(&registration.channel_login)
         .await?
         .ok_or("watched Twitch channel was not found")?;
      let account = self.auth.ensure_eventsub_account().await?;
      let (mut socket, _) = connect_async(EVENTSUB_URL)
         .await
         .map_err(|e| format!("EventSub websocket connect failed: {e}"))?;
      let welcome = socket
         .next()
         .await
         .ok_or("EventSub websocket closed before welcome")?
         .map_err(|e| format!("EventSub welcome failed: {e}"))?;
      let welcome: Envelope = serde_json::from_str(
         welcome
            .to_text()
            .map_err(|e| format!("invalid EventSub welcome: {e}"))?,
      )
      .map_err(|e| format!("invalid EventSub welcome: {e}"))?;
      let session_id = welcome
         .payload
         .session
         .as_ref()
         .and_then(|session| session.id.clone())
         .ok_or("EventSub welcome omitted session id")?;
      let mut receive_timeout = negotiated_receive_timeout(welcome.payload.session.as_ref());
      let subscription_id =
         create_raid_subscription(&self.auth, &account.access_token, &session_id, &identity.id)
            .await?;

      let mut seen = HashSet::new();
      loop {
         if !self.registration_matches(ticket, &sender).await {
            delete_subscription(&self.auth, &account.access_token, &subscription_id).await;
            let _ = socket.close(None).await;
            break;
         }
         let message = match tokio::time::timeout(receive_timeout, socket.next()).await {
            Ok(Some(message)) => message,
            Ok(None) => return Err("EventSub websocket closed".to_string()),
            Err(_) => return Err("EventSub websocket timed out".to_string()),
         };
         let message = message.map_err(|e| format!("EventSub receive failed: {e}"))?;
         let Ok(text) = message.to_text() else {
            continue;
         };
         let Ok(envelope) = serde_json::from_str::<Envelope>(text) else {
            continue;
         };
         if envelope.metadata.message_type.as_deref() == Some("session_reconnect") {
            let reconnect_url = envelope
               .payload
               .session
               .and_then(|session| session.reconnect_url)
               .ok_or("EventSub reconnect omitted URL")?;
            let (mut replacement, _) = connect_async(reconnect_url)
               .await
               .map_err(|e| format!("EventSub reconnect failed: {e}"))?;
            let replacement_welcome = replacement
               .next()
               .await
               .ok_or("EventSub reconnect closed before welcome")?
               .map_err(|e| format!("EventSub reconnect welcome failed: {e}"))?;
            let replacement_welcome: Envelope = serde_json::from_str(
               replacement_welcome
                  .to_text()
                  .map_err(|e| format!("invalid EventSub reconnect welcome: {e}"))?,
            )
            .map_err(|e| format!("invalid EventSub reconnect welcome: {e}"))?;
            receive_timeout =
               negotiated_receive_timeout(replacement_welcome.payload.session.as_ref());
            socket = replacement;
            continue;
         }
         let Some(raid) = raid_from_envelope(envelope, ticket, &identity.id, &mut seen) else {
            continue;
         };
         self.grants.write().await.insert(
            (
               ticket.to_string(),
               registration.session_token.clone(),
               raid.to_broadcaster_user_login.clone(),
            ),
            crate::util::time::now_unix_secs().saturating_add(120),
         );
         let _ = sender.send(raid);
      }
      Ok(())
   }

   async fn registration_matches(
      &self,
      ticket: &str,
      sender: &broadcast::Sender<RaidEvent>,
   ) -> bool {
      self
         .registrations
         .read()
         .await
         .get(ticket)
         .is_some_and(|registration| registration.sender.same_channel(sender))
   }
}

fn next_reconnect_delay(current: Duration) -> Duration {
   current.saturating_mul(2).min(MAX_RECONNECT_DELAY)
}

fn negotiated_receive_timeout(session: Option<&EventSubSession>) -> Duration {
   session
      .and_then(|session| session.keepalive_timeout_seconds)
      .map_or(DEFAULT_KEEPALIVE_TIMEOUT, |seconds| {
         Duration::from_secs(seconds.max(1)).saturating_add(KEEPALIVE_TIMEOUT_GRACE)
      })
}

fn release_registration(
   registrations: &mut HashMap<String, Registration>,
   ticket: &str,
   session_token: &str,
) {
   if let Some(registration) = registrations.get_mut(ticket)
      && registration.session_token == session_token
   {
      registration.receivers = registration.receivers.saturating_sub(1);
      if registration.receivers == 0 {
         registrations.remove(ticket);
      }
   }
}

fn raid_from_envelope(
   envelope: Envelope,
   ticket: &str,
   expected_broadcaster_id: &str,
   seen: &mut HashSet<String>,
) -> Option<RaidEvent> {
   if envelope.metadata.message_type.as_deref() != Some("notification")
      || envelope.metadata.subscription_type.as_deref() != Some("channel.raid")
      || envelope.metadata.subscription_version.as_deref() != Some("1")
   {
      return None;
   }
   let message_id = envelope.metadata.message_id?;
   if !seen.insert(message_id.clone()) {
      return None;
   }
   let event = envelope.payload.event?;
   if event.from_broadcaster_user_id != expected_broadcaster_id {
      return None;
   }
   Some(RaidEvent {
      event_id:                    message_id,
      watch_ticket:                ticket.to_string(),
      from_broadcaster_user_id:    event.from_broadcaster_user_id,
      from_broadcaster_user_login: event.from_broadcaster_user_login,
      to_broadcaster_user_id:      event.to_broadcaster_user_id,
      to_broadcaster_user_login:   event.to_broadcaster_user_login.to_ascii_lowercase(),
      to_broadcaster_user_name:    event.to_broadcaster_user_name,
      viewers:                     event.viewers,
   })
}

async fn create_raid_subscription(
   auth: &TwitchAuthService,
   token: &str,
   session_id: &str,
   broadcaster_id: &str,
) -> Result<String, String> {
   let response = auth
      .api_client()
      .post("https://api.twitch.tv/helix/eventsub/subscriptions")
      .header("Client-Id", auth.client_id())
      .header("Authorization", format!("Bearer {token}"))
      .json(&serde_json::json!({
         "type": "channel.raid",
         "version": "1",
         "condition": { "from_broadcaster_user_id": broadcaster_id },
         "transport": { "method": "websocket", "session_id": session_id }
      }))
      .send()
      .await
      .map_err(|e| format!("EventSub subscription request failed: {e}"))?;
   if response.status().is_success() {
      let payload: SubscriptionResponse = response
         .json()
         .await
         .map_err(|e| format!("EventSub subscription response was invalid: {e}"))?;
      payload
         .data
         .into_iter()
         .next()
         .map(|item| item.id)
         .ok_or_else(|| "EventSub subscription response omitted id".to_string())
   } else {
      Err(format!(
         "EventSub subscription request failed with status {}",
         response.status()
      ))
   }
}

async fn delete_subscription(auth: &TwitchAuthService, token: &str, subscription_id: &str) {
   let result = auth
      .api_client()
      .delete("https://api.twitch.tv/helix/eventsub/subscriptions")
      .header("Client-Id", auth.client_id())
      .header("Authorization", format!("Bearer {token}"))
      .query(&[("id", subscription_id)])
      .send()
      .await;
   if let Err(error) = result {
      tracing::debug!(error = %error, subscription_id, "failed deleting raid EventSub subscription");
   }
}

#[derive(Debug, Deserialize)]
struct SubscriptionResponse {
   data: Vec<SubscriptionItem>,
}

#[derive(Debug, Deserialize)]
struct SubscriptionItem {
   id: String,
}

#[derive(Debug, Deserialize)]
struct Envelope {
   #[serde(default)]
   metadata: Metadata,
   #[serde(default)]
   payload:  Payload,
}

#[derive(Debug, Default, Deserialize)]
struct Metadata {
   message_id:           Option<String>,
   message_type:         Option<String>,
   subscription_type:    Option<String>,
   subscription_version: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct Payload {
   session: Option<EventSubSession>,
   event:   Option<RaidPayload>,
}

#[derive(Debug, Deserialize)]
struct EventSubSession {
   id:                        Option<String>,
   reconnect_url:             Option<String>,
   keepalive_timeout_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct RaidPayload {
   from_broadcaster_user_id:    String,
   from_broadcaster_user_login: String,
   to_broadcaster_user_id:      String,
   to_broadcaster_user_login:   String,
   to_broadcaster_user_name:    String,
   viewers:                     u64,
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn parses_channel_raid_notification() {
      let envelope: Envelope = serde_json::from_value(serde_json::json!({
         "metadata": { "message_id": "event-1", "message_type": "notification", "subscription_type": "channel.raid", "subscription_version": "1" },
         "payload": { "event": {
            "from_broadcaster_user_id": "1", "from_broadcaster_user_login": "streamera",
            "to_broadcaster_user_id": "2", "to_broadcaster_user_login": "StreamerB",
            "to_broadcaster_user_name": "Streamer B", "viewers": 42
         }}
      }))
      .expect("valid raid envelope");
      let event = envelope.payload.event.expect("raid event");
      assert_eq!(event.from_broadcaster_user_id, "1");
      assert_eq!(event.to_broadcaster_user_login, "StreamerB");
      assert_eq!(event.viewers, 42);
   }

   #[test]
   fn malformed_notification_is_rejected() {
      assert!(
         serde_json::from_value::<Envelope>(serde_json::json!({
            "metadata": { "message_type": "notification" },
            "payload": { "event": { "viewers": "many" } }
         }))
         .is_err()
      );
   }

   #[test]
   fn non_raid_subscription_is_identifiable() {
      let envelope: Envelope = serde_json::from_value(serde_json::json!({
         "metadata": { "message_id": "event-2", "message_type": "notification", "subscription_type": "stream.online", "subscription_version": "1" },
         "payload": { "event": {
            "from_broadcaster_user_id": "1", "from_broadcaster_user_login": "streamera",
            "to_broadcaster_user_id": "2", "to_broadcaster_user_login": "streamerb",
            "to_broadcaster_user_name": "Streamer B", "viewers": 42
         }}
      }))
      .expect("syntactically valid envelope");
      assert_ne!(
         envelope.metadata.subscription_type.as_deref(),
         Some("channel.raid")
      );
   }

   #[test]
   fn reconnect_delay_uses_bounded_exponential_backoff() {
      assert_eq!(
         next_reconnect_delay(Duration::from_secs(2)),
         Duration::from_secs(4)
      );
      assert_eq!(
         next_reconnect_delay(Duration::from_secs(32)),
         MAX_RECONNECT_DELAY
      );
      assert_eq!(
         next_reconnect_delay(MAX_RECONNECT_DELAY),
         MAX_RECONNECT_DELAY
      );
   }

   #[test]
   fn receive_timeout_uses_welcome_negotiation() {
      let session = EventSubSession {
         id:                        Some("session".to_string()),
         reconnect_url:             None,
         keepalive_timeout_seconds: Some(10),
      };
      assert_eq!(
         negotiated_receive_timeout(Some(&session)),
         Duration::from_secs(12)
      );
      assert_eq!(negotiated_receive_timeout(None), DEFAULT_KEEPALIVE_TIMEOUT);
   }

   #[test]
   fn registration_remains_until_last_receiver_disconnects() {
      let (sender, _receiver) = broadcast::channel(1);
      let mut registrations = HashMap::from([("ticket".to_string(), Registration {
         session_token: "session".to_string(),
         channel_login: "channel".to_string(),
         sender,
         receivers: 2,
      })]);

      release_registration(&mut registrations, "ticket", "session");
      assert_eq!(registrations["ticket"].receivers, 1);

      release_registration(&mut registrations, "ticket", "session");
      assert!(!registrations.contains_key("ticket"));
   }
}
