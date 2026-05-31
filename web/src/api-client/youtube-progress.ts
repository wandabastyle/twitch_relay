import { isObject, readApiError, request, safeJson } from './core.js';
import type { YouTubeEmbedConfig, YouTubeVideoMeta, YouTubeWatchProgress } from './types.js';

const isYouTubeWatchProgress = (value: unknown): value is YouTubeWatchProgress =>
  isObject(value) &&
  typeof value.video_id === 'string' &&
  typeof value.completed === 'boolean' &&
  typeof value.invidious_sync_attempted === 'boolean' &&
  (value.position_secs === null || typeof value.position_secs === 'number') &&
  (value.duration_secs === null || typeof value.duration_secs === 'number') &&
  (value.updated_at_unix === null || typeof value.updated_at_unix === 'number') &&
  (value.invidious_sync_ok === null || typeof value.invidious_sync_ok === 'boolean') &&
  (value.invidious_sync_action === 'mark_watched' ||
    value.invidious_sync_action === 'mark_unwatched' ||
    value.invidious_sync_action === 'none');

export const getYouTubeEmbedConfig = async (): Promise<YouTubeEmbedConfig> => {
  const response = await request('/api/youtube/embed-config');
  if (!response.ok) {
    throw new Error(readApiError(await safeJson(response)));
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
  return {
    defaults: {
      autoplay: payload.defaults.autoplay,
      quality: payload.defaults.quality,
      quality_dash: payload.defaults.quality_dash,
    },
    invidious_base_url: payload.invidious_base_url,
    referrer_policy: payload.referrer_policy,
  };
};

export const getYouTubeThumbnailUrl = (videoId: string): string =>
  `/api/youtube/thumbnail/${encodeURIComponent(videoId)}`;

export const getYouTubePlaylistThumbnailUrl = (playlistId: string): string =>
  `/api/youtube/playlist-thumbnail/${encodeURIComponent(playlistId)}`;

export const getYouTubeVideoMeta = async (videoId: string): Promise<YouTubeVideoMeta> => {
  const response = await request(`/api/youtube/video/${encodeURIComponent(videoId)}/meta`);
  if (!response.ok) {
    throw new Error(readApiError(await safeJson(response)));
  }
  const payload = await safeJson(response);
  if (
    !isObject(payload) ||
    !isObject(payload.video) ||
    typeof payload.video.duration !== 'number' ||
    typeof payload.video.title !== 'string'
  ) {
    throw new Error('youtube video meta payload is invalid');
  }
  return { duration: payload.video.duration, title: payload.video.title };
};

export const getYouTubeVideoProgress = async (videoId: string): Promise<YouTubeWatchProgress> => {
  const response = await request(`/api/youtube/video/${encodeURIComponent(videoId)}/progress`);
  if (!response.ok) {
    throw new Error(readApiError(await safeJson(response)));
  }
  const payload = await safeJson(response);
  if (!isYouTubeWatchProgress(payload)) {
    throw new Error('youtube watch progress payload is invalid');
  }
  return payload;
};

export const saveYouTubeVideoProgress = async (
  videoId: string,
  progress: {
    readonly completed?: boolean;
    readonly duration_secs?: number | null;
    readonly position_secs: number;
  },
): Promise<YouTubeWatchProgress> => {
  const response = await request(`/api/youtube/video/${encodeURIComponent(videoId)}/progress`, {
    body: JSON.stringify(progress),
    headers: { 'content-type': 'application/json' },
    method: 'PUT',
  });
  if (!response.ok) {
    throw new Error(readApiError(await safeJson(response)));
  }
  const payload = await safeJson(response);
  if (!isYouTubeWatchProgress(payload)) {
    throw new Error('youtube watch progress payload is invalid');
  }
  return payload;
};
