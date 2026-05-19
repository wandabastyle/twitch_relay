import { isObject, safeJson, readApiError, request } from './core';
import type {
  YoutubeChannel,
  YoutubeChannelInfo,
  YoutubeVideo,
  YouTubeEmbedConfig,
  YouTubeVideoMeta,
  YouTubeWatchProgress,
  YoutubePlaylist,
} from './types';

const channelVideosCache = new Map<string, YoutubeVideo[]>();
const playlistVideosCache = new Map<string, YoutubeVideo[]>();

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
  const response = await request('/api/youtube/subscriptions');
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !Array.isArray(payload.channels)) {
    throw new Error('subscriptions payload is invalid');
  }

  return payload.channels as YoutubeChannel[];
}

export async function getYouTubeRecentVideos(maxResults = 25): Promise<YoutubeVideo[]> {
  const params = new URLSearchParams();
  params.set('max_results', String(maxResults));

  const response = await request(`/api/youtube/recent?${params.toString()}`);
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !Array.isArray(payload.videos)) {
    throw new Error('recent videos payload is invalid');
  }

  return payload.videos as YoutubeVideo[];
}

export async function getYouTubeChannelInfo(channelId: string): Promise<YoutubeChannelInfo> {
  const url = `/api/youtube/channel/${encodeURIComponent(channelId)}/info`;
  const response = await request(url);

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !isObject(payload.channel)) {
    throw new Error('channel info payload is invalid');
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
    params.set('max_results', String(maxResults));
  }

  const url = `/api/youtube/channel/${encodeURIComponent(channelId)}/videos?${params.toString()}`;
  const response = await request(url);

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !Array.isArray(payload.videos)) {
    throw new Error('channel videos payload is invalid');
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
    params.set('max_results', String(maxResults));
  }

  const url = `/api/youtube/channel/${encodeURIComponent(channelId)}/videos?${params.toString()}`;
  const response = await request(url);

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !Array.isArray(payload.videos)) {
    throw new Error('channel videos payload is invalid');
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
  const response = await request('/api/youtube/embed-config');
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (
    !isObject(payload) ||
    typeof payload.invidious_base_url !== 'string' ||
    !isObject(payload.defaults) ||
    typeof payload.defaults.autoplay !== 'number' ||
    typeof payload.defaults.quality !== 'string' ||
    typeof payload.defaults.quality_dash !== 'string' ||
    typeof payload.referrer_policy !== 'string'
  ) {
    throw new Error('youtube embed config payload is invalid');
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

  return config;
}

export function getYouTubeThumbnailUrl(videoId: string): string {
  return `/api/youtube/thumbnail/${encodeURIComponent(videoId)}`;
}

export function getYouTubePlaylistThumbnailUrl(playlistId: string): string {
  return `/api/youtube/playlist-thumbnail/${encodeURIComponent(playlistId)}`;
}

export async function getYouTubeVideoMeta(videoId: string): Promise<YouTubeVideoMeta> {
  const response = await request(`/api/youtube/video/${encodeURIComponent(videoId)}/meta`);
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (
    !isObject(payload) ||
    !isObject(payload.video) ||
    typeof payload.video.title !== 'string' ||
    typeof payload.video.duration !== 'number'
  ) {
    throw new Error('youtube video meta payload is invalid');
  }

  return {
    title: payload.video.title,
    duration: payload.video.duration,
  };
}

export async function getYouTubeVideoProgress(videoId: string): Promise<YouTubeWatchProgress> {
  const response = await request(`/api/youtube/video/${encodeURIComponent(videoId)}/progress`);
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (
    !isObject(payload) ||
    typeof payload.video_id !== 'string' ||
    (payload.position_secs !== null && typeof payload.position_secs !== 'number') ||
    (payload.duration_secs !== null && typeof payload.duration_secs !== 'number') ||
    (payload.updated_at_unix !== null && typeof payload.updated_at_unix !== 'number') ||
    typeof payload.completed !== 'boolean' ||
    typeof payload.invidious_sync_attempted !== 'boolean' ||
    (payload.invidious_sync_ok !== null && typeof payload.invidious_sync_ok !== 'boolean') ||
    (payload.invidious_sync_action !== 'mark_watched' &&
      payload.invidious_sync_action !== 'mark_unwatched' &&
      payload.invidious_sync_action !== 'none')
  ) {
    throw new Error('youtube watch progress payload is invalid');
  }

  return payload as unknown as YouTubeWatchProgress;
}

export async function saveYouTubeVideoProgress(
  videoId: string,
  progress: {
    position_secs: number;
    duration_secs?: number | null;
    completed?: boolean;
  },
): Promise<YouTubeWatchProgress> {
  const response = await request(`/api/youtube/video/${encodeURIComponent(videoId)}/progress`, {
    method: 'PUT',
    headers: {
      'content-type': 'application/json',
    },
    body: JSON.stringify(progress),
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (
    !isObject(payload) ||
    typeof payload.video_id !== 'string' ||
    (payload.position_secs !== null && typeof payload.position_secs !== 'number') ||
    (payload.duration_secs !== null && typeof payload.duration_secs !== 'number') ||
    (payload.updated_at_unix !== null && typeof payload.updated_at_unix !== 'number') ||
    typeof payload.completed !== 'boolean' ||
    typeof payload.invidious_sync_attempted !== 'boolean' ||
    (payload.invidious_sync_ok !== null && typeof payload.invidious_sync_ok !== 'boolean') ||
    (payload.invidious_sync_action !== 'mark_watched' &&
      payload.invidious_sync_action !== 'mark_unwatched' &&
      payload.invidious_sync_action !== 'none')
  ) {
    throw new Error('youtube watch progress payload is invalid');
  }

  return payload as unknown as YouTubeWatchProgress;
}

export function getCachedPlaylistVideos(playlistId: string): YoutubeVideo[] | undefined {
  return playlistVideosCache.get(playlistId);
}

export function setCachedPlaylistVideos(playlistId: string, videos: YoutubeVideo[]): void {
  playlistVideosCache.set(playlistId, videos);
}

export function clearPlaylistVideosCache(playlistId?: string): void {
  if (playlistId) {
    playlistVideosCache.delete(playlistId);
  } else {
    playlistVideosCache.clear();
  }
}

export async function getYouTubePlaylists(): Promise<YoutubePlaylist[]> {
  const response = await request('/api/youtube/playlists');
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !Array.isArray(payload.playlists)) {
    throw new Error('playlists payload is invalid');
  }

  return payload.playlists as YoutubePlaylist[];
}

export async function getYouTubePlaylistVideos(
  playlistId: string,
): Promise<{ videos: YoutubeVideo[]; fromCache: boolean }> {
  const cached = playlistVideosCache.get(playlistId);
  if (cached) {
    return { videos: cached, fromCache: true };
  }

  const url = `/api/youtube/playlist/${encodeURIComponent(playlistId)}/videos`;
  const response = await request(url);

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !Array.isArray(payload.videos)) {
    throw new Error('playlist videos payload is invalid');
  }

  const videos = payload.videos as YoutubeVideo[];
  setCachedPlaylistVideos(playlistId, videos);
  return { videos, fromCache: false };
}
