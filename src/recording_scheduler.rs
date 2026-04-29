use std::{collections::HashMap, time::Duration};

use tokio::time;

use crate::{
    config::RecordingConfig,
    live_status::LiveStatusService,
    recording::{RecordingMode, RecordingService},
    recording_rules,
};

#[derive(Debug, Default, Clone)]
struct RuleState {
    live_confirmations: u64,
    offline_confirmations: u64,
}

#[derive(Debug, Clone)]
pub struct RecordingScheduler;

impl RecordingScheduler {
    pub fn start(
        config: RecordingConfig,
        live_status: LiveStatusService,
        service: RecordingService,
    ) {
        tokio::spawn(async move {
            let mut tick = time::interval(Duration::from_secs(config.poll_interval_secs));
            let mut state_by_channel: HashMap<String, RuleState> = HashMap::new();

            loop {
                tick.tick().await;

                let rules = match recording_rules::load_rules() {
                    Ok(rules) => rules,
                    Err(error) => {
                        tracing::warn!(error = %error, "recording scheduler failed to load rules");
                        continue;
                    }
                };

                let enabled_rules: Vec<_> = rules.into_iter().filter(|rule| rule.enabled).collect();
                if enabled_rules.is_empty() {
                    continue;
                }

                let channels: Vec<String> = enabled_rules
                    .iter()
                    .map(|rule| rule.channel_login.trim().to_ascii_lowercase())
                    .filter(|login| !login.is_empty())
                    .collect();

                let response = live_status.check_multiple(&channels).await;

                for rule in enabled_rules {
                    let login = rule.channel_login.trim().to_ascii_lowercase();
                    if login.is_empty() {
                        continue;
                    }

                    let Some(channel_status) = response.channels.get(&login) else {
                        tracing::warn!(channel = %login, "recording scheduler missing live status response");
                        continue;
                    };

                    let state = state_by_channel.entry(login.clone()).or_default();

                    if channel_status.live {
                        state.live_confirmations = state.live_confirmations.saturating_add(1);
                        state.offline_confirmations = 0;
                    } else {
                        state.offline_confirmations = state.offline_confirmations.saturating_add(1);
                        state.live_confirmations = 0;
                    }

                    let active = service.get_active_recording(&login).await;

                    if active.is_none()
                        && channel_status.live
                        && state.live_confirmations >= config.start_live_confirmations
                    {
                        let quality = match RecordingService::validate_quality(&rule.quality) {
                            Ok(value) => value,
                            Err(_) => config.default_quality.clone(),
                        };

                        if let Err(error) = service
                            .start_recording(
                                &login,
                                &quality,
                                RecordingMode::Auto,
                                channel_status.title.as_deref(),
                            )
                            .await
                        {
                            tracing::warn!(channel = %login, error = %error, "auto recording start failed");
                        }
                    }

                    if let Some(active_recording) = active {
                        if active_recording.mode != RecordingMode::Auto {
                            continue;
                        }

                        let max_minutes = rule.max_duration_minutes.or(config.max_duration_minutes);
                        if let Some(limit) = max_minutes {
                            let elapsed_secs =
                                now_unix().saturating_sub(active_recording.started_at_unix);
                            if elapsed_secs >= limit.saturating_mul(60) {
                                if let Err(error) = service.stop_recording(&login).await {
                                    tracing::warn!(channel = %login, error = %error, "auto recording max-duration stop failed");
                                }
                                continue;
                            }
                        }

                        if rule.stop_when_offline
                            && !channel_status.live
                            && state.offline_confirmations >= config.stop_offline_confirmations
                            && let Err(error) = service.stop_recording(&login).await
                        {
                            tracing::warn!(channel = %login, error = %error, "auto recording offline stop failed");
                        }
                    }
                }
            }
        });
    }
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
