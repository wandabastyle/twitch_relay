// ============================================
// Auth
// ============================================

export interface SessionStateResponse {
  authenticated: boolean;
}

export interface QrSessionResponse {
  token: string;
  expires_at: number;
}

export interface QrStatusResponse {
  status: "pending" | "authenticated";
}

// ============================================
// App/version
// ============================================

export interface VersionResponse {
  version: string;
}

// ============================================
// Twitch channels
// ============================================

export interface ChannelEntry {
  login: string;
  image_url?: string;
  display_name?: string;
  source: "manual" | "followed" | "both";
  removable: boolean;
}

export interface ChannelsResponse {
  channels: Array<ChannelEntry>;
}

// ============================================
// Live status
// ============================================

export interface ChannelStatus {
  live: boolean;
  viewer_count?: number;
  game?: string;
  title?: string;
  profile_url?: string;
  display_name?: string;
}

export interface LiveStatusResponse {
  channels: Record<string, ChannelStatus>;
}

// ============================================
// Twitch OAuth
// ============================================

export interface TwitchStatusResponse {
  connected: boolean;
  login?: string;
  display_name?: string;
  scopes: string[];
}

// ============================================
// Recordings
// ============================================

export type RecordingMode = "manual" | "auto";

export interface ActiveRecording {
  channel_login: string;
  quality: string;
  started_at_unix: number;
  output_path: string;
  pid?: number;
  mode: RecordingMode;
  error?: string;
}

export interface RecordingFileEntry {
  channel_login: string;
  filename: string;
  path_display: string;
  status: string;
  pinned: boolean;
  has_hls: boolean;
  processing_state: "processing" | "ready";
}

export type RecordingJobKind = "merge" | "finalize";

export interface RecordingJobStartResponse {
  job_id: string;
  kind: RecordingJobKind;
  channel_login: string;
  expected_filename: string;
  source_count: number;
}

export interface RecordingJobStatusResponse {
  job_id: string;
  kind: RecordingJobKind;
  status: "queued" | "running" | "completed" | "failed";
  channel_login: string;
  expected_filename: string;
  final_filename: string | null;
  error: string | null;
}

export interface RecordingsResponse {
  active: Array<ActiveRecording>;
  completed: Array<RecordingFileEntry>;
  incomplete: Array<RecordingFileEntry>;
}

export interface RecordingWatchProgress {
  channel_login: string;
  filename: string;
  position_secs: number | null;
  duration_secs: number | null;
  updated_at_unix: number | null;
  completed: boolean;
}

// ============================================
// Recording rules
// ============================================

export interface RecordingRule {
  channel_login: string;
  enabled: boolean;
  quality: string;
  stop_when_offline: boolean;
  max_duration_minutes: number | null;
  keep_last_videos: number | null;
}

// ============================================
// YouTube/Invidious
// ============================================

export interface YoutubeChannel {
  name: string;
  channel_id: string;
  url: string;
  avatar?: string;
  description?: string;
}

export interface YoutubeChannelInfo {
  name: string;
  channel_id: string;
  url: string;
  description?: string;
  description_html?: string;
  sub_count: number;
  author_verified: boolean;
  avatar?: string;
}

export interface YoutubeVideo {
  title: string;
  video_id: string;
  author: string;
  author_id: string;
  published: number;
  published_text: string;
  duration: number;
  thumbnail: string;
  view_count: number;
  description?: string;
}

export interface YouTubeEmbedConfig {
  invidious_base_url: string;
  defaults: {
    autoplay: number;
    quality: string;
    quality_dash: string;
  };
  referrer_policy: string;
}

export interface YouTubeVideoMeta {
  title: string;
  duration: number;
}

export interface YouTubeWatchProgress {
  video_id: string;
  position_secs: number | null;
  duration_secs: number | null;
  updated_at_unix: number | null;
  completed: boolean;
  invidious_sync_attempted: boolean;
  invidious_sync_ok: boolean | null;
  invidious_sync_action: "mark_watched" | "mark_unwatched" | "none";
}

export interface YoutubePlaylist {
  title: string;
  playlist_id: string;
  video_count: number;
  updated: number;
  thumbnail?: string;
}

// ============================================
// Stream/watch
// ============================================

export interface WatchTicketResponse {
  watch_url: string;
}

export interface WatchSessionResponse {
  channel: string;
  manifest_url: string;
  relay: boolean;
  app_version: string;
}
