use std::time::Duration;

use tokio::{
   sync::mpsc,
   time,
};

use crate::{
   channel_catalog::ChannelCatalogService,
   chat::ChatService,
   live_status::LiveStatusService,
   stream_proxy::StreamSessionService,
};

#[derive(Debug, Clone)]
pub struct PrewarmCoordinator {
   trigger_tx: mpsc::UnboundedSender<()>,
}

impl PrewarmCoordinator {
   pub fn new(
      catalog: ChannelCatalogService,
      live_status: LiveStatusService,
      chat: ChatService,
      stream: StreamSessionService,
   ) -> Self {
      let (trigger_tx, mut trigger_rx) = mpsc::unbounded_channel::<()>();

       tokio::spawn(async move {
          let mut live_tick = time::interval(Duration::from_mins(1));
          let mut emote_tick = time::interval(Duration::from_mins(15));

          loop {
             tokio::select! {
                 _ = live_tick.tick() => {
                     prewarm_live_status(&catalog, &live_status, &stream).await;
                 }
                 _ = emote_tick.tick() => {
                     prewarm_emotes(&catalog, &chat).await;
                 }
                 trigger = trigger_rx.recv() => {
                     if trigger.is_none() {
                         break;
                     }
                     prewarm_live_status(&catalog, &live_status, &stream).await;
                     prewarm_emotes(&catalog, &chat).await;
                 }
             }
          }
       });

      Self { trigger_tx }
   }

   pub fn trigger_now(&self) {
      let _ = self.trigger_tx.send(());
   }
}

async fn prewarm_live_status(
   catalog: &ChannelCatalogService,
   live_status: &LiveStatusService,
   stream: &StreamSessionService,
) {
   let channels = catalog.channel_logins().await;
   if channels.is_empty() {
      return;
   }

   let timeout = Duration::from_secs(25);
   match time::timeout(timeout, live_status.check_multiple(&channels)).await {
      Ok(response) => {
         for (login, status) in response.channels {
            if status.live {
               // For every live channel, maintain prewarm
               stream.maintain_prewarm_for_live_channel(&login).await;
            } else {
               // For offline channels, drop any existing prewarm
               stream.drop_prewarm_for_offline_channel(&login).await;
            }
         }
      },
      Err(_) => {
         tracing::debug!("live status prewarm timed out");
      },
   }
}

async fn prewarm_emotes(catalog: &ChannelCatalogService, chat: &ChatService) {
   let channels = catalog.channel_logins().await;
   if channels.is_empty() {
      return;
   }

   if let Err(error) = chat.prewarm_emotes_for_channels(&channels).await {
      tracing::debug!(error = %error, "chat emote prewarm skipped");
   }
}
