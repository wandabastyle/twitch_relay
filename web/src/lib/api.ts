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

interface ErrorPayload {
  error?: string;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readError(payload: unknown): string {
  if (isObject(payload) && typeof payload.error === "string") {
    return payload.error;
  }
  return "request failed";
}

async function safeJson(response: Response): Promise<unknown> {
  try {
    return (await response.json()) as unknown;
  } catch {
    return null;
  }
}

async function request(input: string, init?: RequestInit): Promise<Response> {
  return fetch(input, {
    credentials: "same-origin",
    ...init,
  });
}

export async function getSessionState(): Promise<boolean> {
  const response = await request("/auth/session");
  if (!response.ok) {
    throw new Error(`session request failed (${String(response.status)})`);
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.authenticated !== "boolean") {
    throw new Error("session response payload is invalid");
  }

  return payload.authenticated;
}

export async function login(accessCode: string, qrToken?: string): Promise<void> {
  const body: Record<string, string> = { access_code: accessCode };
  if (qrToken) {
    body.qr_token = qrToken;
  }

  const response = await request("/auth/login", {
    method: "POST",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify(body),
  });

  if (response.ok) {
    return;
  }

  const payload = (await safeJson(response)) as ErrorPayload;
  throw new Error(payload.error ?? "login failed");
}

export async function createQrSession(): Promise<QrSessionResponse> {
  const response = await request("/auth/qr/create");
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (
    !isObject(payload) ||
    typeof payload.token !== "string" ||
    typeof payload.expires_at !== "number"
  ) {
    throw new Error("qr session response payload is invalid");
  }

  return {
    token: payload.token,
    expires_at: payload.expires_at,
  };
}

export async function getQrStatus(token: string): Promise<QrStatusResponse> {
  const response = await request(`/auth/qr/status/${encodeURIComponent(token)}`);
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.status !== "string") {
    throw new Error("qr status response payload is invalid");
  }

  return {
    status: payload.status === "authenticated" ? "authenticated" : "pending",
  };
}

export async function claimQrSession(token: string): Promise<QrStatusResponse> {
  const response = await request(`/auth/qr/claim/${encodeURIComponent(token)}`, {
    method: "POST",
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.status !== "string") {
    throw new Error("qr claim response payload is invalid");
  }

  return {
    status: payload.status === "authenticated" ? "authenticated" : "pending",
  };
}

export async function logout(): Promise<void> {
  await request("/auth/logout", { method: "POST" });
}

export async function getChannels(): Promise<Array<ChannelEntry>> {
  const response = await request("/api/channels");
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !Array.isArray(payload.channels)) {
    throw new Error("channels payload is invalid");
  }

  const channels = payload.channels.filter(
    (item): item is ChannelEntry =>
      isObject(item) &&
      typeof item.login === "string" &&
      (item.source === "manual" || item.source === "followed" || item.source === "both") &&
      typeof item.removable === "boolean",
  );

  return channels;
}

export async function createWatchTicket(channelLogin: string): Promise<WatchTicketResponse> {
  const response = await request("/api/watch-ticket", {
    method: "POST",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify({ channel_login: channelLogin }),
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.watch_url !== "string") {
    throw new Error("watch ticket payload is invalid");
  }

  return {
    watch_url: payload.watch_url,
  };
}

export async function addChannel(login: string): Promise<void> {
  const response = await request("/api/channels", {
    method: "POST",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify({ login }),
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }
}

export async function removeChannel(login: string): Promise<void> {
  const response = await request(`/api/channels/${encodeURIComponent(login)}`, {
    method: "DELETE",
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }
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

interface LiveStatusCacheEntry {
  timestamp: number;
  data: LiveStatusResponse;
}

const LIVE_STATUS_CACHE_KEY = "twitchRelay.liveStatus";
const LIVE_STATUS_CACHE_MAX_AGE_MS = 60000;

function parseLiveStatusPayload(payload: unknown): LiveStatusResponse {
  if (!isObject(payload) || !isObject(payload.channels)) {
    throw new Error("live status payload is invalid");
  }

  return {
    channels: payload.channels as Record<string, ChannelStatus>,
  };
}

function getLiveStatusFromCache(): LiveStatusResponse | null {
  if (typeof window === "undefined") {
    return null;
  }

  try {
    const encoded = window.sessionStorage.getItem(LIVE_STATUS_CACHE_KEY);
    if (!encoded) {
      return null;
    }

    const parsed = JSON.parse(encoded) as unknown;
    if (!isObject(parsed) || typeof parsed.timestamp !== "number" || !("data" in parsed)) {
      return null;
    }

    const ageMs = Date.now() - parsed.timestamp;
    if (ageMs > LIVE_STATUS_CACHE_MAX_AGE_MS) {
      return null;
    }

    return parseLiveStatusPayload(parsed.data);
  } catch {
    return null;
  }
}

function setLiveStatusCache(data: LiveStatusResponse): void {
  if (typeof window === "undefined") {
    return;
  }

  try {
    const payload: LiveStatusCacheEntry = {
      timestamp: Date.now(),
      data,
    };
    window.sessionStorage.setItem(LIVE_STATUS_CACHE_KEY, JSON.stringify(payload));
  } catch {
    // Ignore storage failures and continue with in-memory state.
  }
}

async function fetchLiveStatusFromApi(): Promise<LiveStatusResponse> {
  const response = await request("/api/live-status");
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  return parseLiveStatusPayload(payload);
}

async function refreshLiveStatusCache(): Promise<void> {
  try {
    const fresh = await fetchLiveStatusFromApi();
    setLiveStatusCache(fresh);
  } catch {
    // Keep existing cache if refresh fails.
  }
}

export async function getLiveStatus(): Promise<LiveStatusResponse> {
  const cached = getLiveStatusFromCache();
  if (cached) {
    void refreshLiveStatusCache();
    return cached;
  }

  const fresh = await fetchLiveStatusFromApi();
  setLiveStatusCache(fresh);
  return fresh;
}

export async function getTwitchStatus(): Promise<TwitchStatusResponse> {
  const response = await request("/api/twitch/status");
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.connected !== "boolean") {
    throw new Error("twitch status payload is invalid");
  }

  return {
    connected: payload.connected,
    login: typeof payload.login === "string" ? payload.login : undefined,
    display_name: typeof payload.display_name === "string" ? payload.display_name : undefined,
    scopes: Array.isArray(payload.scopes)
      ? payload.scopes.filter((scope): scope is string => typeof scope === "string")
      : [],
  };
}

export function getTwitchConnectUrl(): string {
  return "/api/twitch/connect";
}

export async function disconnectTwitch(): Promise<void> {
  const response = await request("/api/twitch/disconnect", { method: "POST" });
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }
}

export async function getVersion(): Promise<VersionResponse> {
  const response = await request("/api/version");
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.version !== "string") {
    throw new Error("version payload is invalid");
  }

  return { version: payload.version };
}

export async function getRecordingRules(): Promise<Array<RecordingRule>> {
  const response = await request("/api/recording-rules");
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !Array.isArray(payload.rules)) {
    throw new Error("recording rules payload is invalid");
  }

  return payload.rules as Array<RecordingRule>;
}

export async function upsertRecordingRule(rule: {
  channel_login: string;
  enabled: boolean;
  quality?: string;
  stop_when_offline?: boolean;
  max_duration_minutes?: number | null;
  keep_last_videos?: number | null;
}): Promise<RecordingRule> {
  const response = await request("/api/recording-rules", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(rule),
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  return (await safeJson(response)) as RecordingRule;
}

export async function getRecordings(): Promise<RecordingsResponse> {
  const response = await request("/api/recordings");
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (
    !isObject(payload) ||
    !Array.isArray(payload.active) ||
    !Array.isArray(payload.completed) ||
    !Array.isArray(payload.incomplete)
  ) {
    throw new Error("recordings payload is invalid");
  }

  return {
    active: payload.active as Array<ActiveRecording>,
    completed: payload.completed as Array<RecordingFileEntry>,
    incomplete: payload.incomplete as Array<RecordingFileEntry>,
  };
}

export async function startRecording(
  channel_login: string,
  quality?: string,
  stream_title?: string,
): Promise<void> {
  const response = await request("/api/recordings/start", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ channel_login, quality, stream_title }),
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }
}

export async function stopRecording(channel_login: string): Promise<void> {
  const response = await request("/api/recordings/stop", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ channel_login }),
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }
}

export async function deleteRecordingFile(payload: {
  bucket: "completed" | "incomplete";
  channel_login: string;
  filename: string;
}): Promise<void> {
  const response = await request("/api/recordings/delete", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readError(body));
  }
}

export async function pinRecordingFile(payload: {
  bucket: "completed";
  channel_login: string;
  filename: string;
}): Promise<void> {
  const response = await request("/api/recordings/pin", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readError(body));
  }
}

export async function unpinRecordingFile(payload: {
  bucket: "completed";
  channel_login: string;
  filename: string;
}): Promise<void> {
  const response = await request("/api/recordings/unpin", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readError(body));
  }
}

// ============ YouTube / Invidious API ============

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

const channelVideosCache = new Map<string, YoutubeVideo[]>();

export function getCachedChannelVideos(channelId: string): YoutubeVideo[] | undefined {
  return channelVideosCache.get(channelId);
}

export function setCachedChannelVideos(channelId: string, videos: YoutubeVideo[]): void {
  channelVideosCache.set(channelId, videos);
}

export function clearChannelVideosCache(channelId?: string): void {
  if (channelId) {
    channelVideosCache.delete(channelId);
  } else {
    channelVideosCache.clear();
  }
}

export async function getYouTubeSubscriptions(): Promise<YoutubeChannel[]> {
  const response = await request("/api/youtube/subscriptions");
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !Array.isArray(payload.channels)) {
    throw new Error("subscriptions payload is invalid");
  }

  return payload.channels as YoutubeChannel[];
}

export async function getYouTubeChannelInfo(channelId: string): Promise<YoutubeChannelInfo> {
  const url = `/api/youtube/channel/${encodeURIComponent(channelId)}/info`;
  const response = await request(url);

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !isObject(payload.channel)) {
    throw new Error("channel info payload is invalid");
  }

  return payload.channel as unknown as YoutubeChannelInfo;
}

function videosAreEqual(a: YoutubeVideo[], b: YoutubeVideo[]): boolean {
  if (a.length !== b.length) return false;
  const aIds = a.map((v) => v.video_id);
  const bIds = b.map((v) => v.video_id);
  return JSON.stringify(aIds) === JSON.stringify(bIds);
}

export async function getYouTubeChannelVideos(
  channelId: string,
  maxResults?: number,
): Promise<{ videos: YoutubeVideo[]; fromCache: boolean }> {
  const cached = channelVideosCache.get(channelId);
  if (cached) {
    return { videos: cached, fromCache: true };
  }

  const params = new URLSearchParams();
  if (maxResults) {
    params.set("max_results", String(maxResults));
  }

  const url = `/api/youtube/channel/${encodeURIComponent(channelId)}/videos?${params.toString()}`;
  const response = await request(url);

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !Array.isArray(payload.videos)) {
    throw new Error("channel videos payload is invalid");
  }

  const videos = payload.videos as YoutubeVideo[];
  setCachedChannelVideos(channelId, videos);
  return { videos, fromCache: false };
}

export async function refreshYouTubeChannelVideos(
  channelId: string,
  maxResults?: number,
): Promise<{ videos: YoutubeVideo[]; changed: boolean }> {
  const params = new URLSearchParams();
  if (maxResults) {
    params.set("max_results", String(maxResults));
  }

  const url = `/api/youtube/channel/${encodeURIComponent(channelId)}/videos?${params.toString()}`;
  const response = await request(url);

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !Array.isArray(payload.videos)) {
    throw new Error("channel videos payload is invalid");
  }

  const freshVideos = payload.videos as YoutubeVideo[];
  const cached = channelVideosCache.get(channelId);

  if (cached && videosAreEqual(cached, freshVideos)) {
    return { videos: cached, changed: false };
  }

  setCachedChannelVideos(channelId, freshVideos);
  return { videos: freshVideos, changed: true };
}

export async function getYouTubeEmbedConfig(): Promise<YouTubeEmbedConfig> {
  const response = await request("/api/youtube/embed-config");
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (
    !isObject(payload) ||
    typeof payload.invidious_base_url !== "string" ||
    !isObject(payload.defaults) ||
    typeof payload.defaults.autoplay !== "number" ||
    typeof payload.defaults.quality !== "string" ||
    typeof payload.defaults.quality_dash !== "string" ||
    typeof payload.referrer_policy !== "string"
  ) {
    throw new Error("youtube embed config payload is invalid");
  }

  const config: YouTubeEmbedConfig = {
    invidious_base_url: payload.invidious_base_url,
    defaults: {
      autoplay: payload.defaults.autoplay,
      quality: payload.defaults.quality,
      quality_dash: payload.defaults.quality_dash,
    },
    referrer_policy: payload.referrer_policy,
  };

  // Optional basic auth credentials
  if (typeof payload.basic_auth_user === "string") {
    config.basic_auth_user = payload.basic_auth_user;
  }
  if (typeof payload.basic_auth_password === "string") {
    config.basic_auth_password = payload.basic_auth_password;
  }

  return config;
}

export function getYouTubeThumbnailUrl(videoId: string): string {
  return `/api/youtube/thumbnail/${encodeURIComponent(videoId)}`;
}

export async function getYouTubeVideoMeta(videoId: string): Promise<YouTubeVideoMeta> {
  const response = await request(`/api/youtube/video/${encodeURIComponent(videoId)}/meta`);
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (
    !isObject(payload) ||
    !isObject(payload.video) ||
    typeof payload.video.title !== "string" ||
    typeof payload.video.duration !== "number"
  ) {
    throw new Error("youtube video meta payload is invalid");
  }

  return {
    title: payload.video.title,
    duration: payload.video.duration,
  };
}
