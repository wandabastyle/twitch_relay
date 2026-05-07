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

export interface WatchTicketResponse {
  watch_url: string;
}

export interface TwitchStatusResponse {
  connected: boolean;
  login?: string;
  display_name?: string;
  scopes: string[];
}

export interface VersionResponse {
  version: string;
}

export type RecordingMode = "manual" | "auto";

export interface RecordingRule {
  channel_login: string;
  enabled: boolean;
  quality: string;
  stop_when_offline: boolean;
  max_duration_minutes: number | null;
  keep_last_videos: number | null;
}

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
}

export interface RecordingsResponse {
  active: Array<ActiveRecording>;
  completed: Array<RecordingFileEntry>;
  incomplete: Array<RecordingFileEntry>;
}

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
  basic_auth_user?: string;
  basic_auth_password?: string;
}

export interface YouTubeVideoMeta {
  title: string;
  duration: number;
}

export interface YoutubePlaylist {
  title: string;
  playlist_id: string;
  video_count: number;
  updated: number;
  thumbnail?: string;
}
