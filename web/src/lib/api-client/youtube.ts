import { isObject, readApiError, request, safeJson } from './core.js';
import type {
  YouTubeEmbedConfig,
  YouTubeVideoMeta,
  YouTubeWatchProgress,
  YoutubeChannel,
  YoutubeChannelInfo,
  YoutubePlaylist,
  YoutubeVideo,
} from './types.js';

const DEFAULT_MAX_RESULTS = 25;
const MIN_RESULTS_THRESHOLD = 0;

const channelVideosCache = new Map<string, YoutubeVideo[]>();
const playlistVideosCache = new Map<string, YoutubeVideo[]>();

const isYoutubeChannel = (value: unknown): value is YoutubeChannel =>
  isObject(value) &&
  typeof value.name === 'string' &&
  typeof value.channel_id === 'string' &&
  typeof value.url === 'string';

const isYoutubeVideo = (value: unknown): value is YoutubeVideo =>
  isObject(value) &&
  typeof value.title === 'string' &&
  typeof value.video_id === 'string' &&
  typeof value.author === 'string' &&
  typeof value.author_id === 'string' &&
  typeof value.published === 'number' &&
  typeof value.published_text === 'string' &&
  typeof value.duration === 'number' &&
  typeof value.thumbnail === 'string' &&
  typeof value.view_count === 'number';

const isYoutubeChannelInfo = (value: unknown): value is YoutubeChannelInfo =>
  isObject(value) &&
  typeof value.name === 'string' &&
  typeof value.channel_id === 'string' &&
  typeof value.url === 'string' &&
  typeof value.sub_count === 'number' &&
  typeof value.author_verified === 'boolean';

const isYoutubePlaylist = (value: unknown): value is YoutubePlaylist =>
  isObject(value) &&
  typeof value.title === 'string' &&
  typeof value.playlist_id === 'string' &&
  typeof value.video_count === 'number' &&
  typeof value.updated === 'number';

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

const validateArray = <T>(
  value: unknown,
  predicate: (item: unknown) => item is T,
): value is readonly T[] => Array.isArray(value) && value.every(predicate);

export const getCachedChannelVideos = (channelId: string): YoutubeVideo[] | null => {
  const cached = channelVideosCache.get(channelId);
  return cached ?? null;
};

export const setCachedChannelVideos = (
  channelId: string,
  videos: readonly Readonly<YoutubeVideo>[],
): void => {
  channelVideosCache.set(channelId, [...videos]);
};

export const clearChannelVideosCache = (channelId: string | null = null): void => {
  if (channelId !== null && channelId !== '') {
    channelVideosCache.delete(channelId);
  } else {
    channelVideosCache.clear();
  }
};

export const getYouTubeSubscriptions = async (): Promise<readonly YoutubeChannel[]> => {
  const response = await request('/api/youtube/subscriptions');
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !validateArray(payload.channels, isYoutubeChannel)) {
    throw new Error('subscriptions payload is invalid');
  }

  return payload.channels;
};

export const getYouTubeRecentVideos = async (
  maxResults = DEFAULT_MAX_RESULTS,
): Promise<readonly YoutubeVideo[]> => {
  const params = new URLSearchParams();
  params.set('max_results', String(maxResults));

  const response = await request(`/api/youtube/recent?${params.toString()}`);
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !validateArray(payload.videos, isYoutubeVideo)) {
    throw new Error('recent videos payload is invalid');
  }

  return payload.videos;
};

export const getYouTubeChannelInfo = async (channelId: string): Promise<YoutubeChannelInfo> => {
  const url = `/api/youtube/channel/${encodeURIComponent(channelId)}/info`;
  const response = await request(url);

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !isYoutubeChannelInfo(payload.channel)) {
    throw new Error('channel info payload is invalid');
  }

  return payload.channel;
};

const videosAreEqual = (
  videosA: readonly Readonly<YoutubeVideo>[],
  videosB: readonly Readonly<YoutubeVideo>[],
): boolean => {
  if (videosA.length !== videosB.length) {
    return false;
  }
  const idsA = videosA.map((video: Readonly<YoutubeVideo>) => video.video_id);
  const idsB = videosB.map((video: Readonly<YoutubeVideo>) => video.video_id);
  return JSON.stringify(idsA) === JSON.stringify(idsB);
};

export const getYouTubeChannelVideos = async (
  channelId: string,
  maxResults: number | null = null,
): Promise<{ fromCache: boolean; videos: readonly YoutubeVideo[] }> => {
  const cached = channelVideosCache.get(channelId);
  if (cached != null) {
    return { fromCache: true, videos: cached };
  }

  const params = new URLSearchParams();
  if (maxResults !== null && maxResults > MIN_RESULTS_THRESHOLD) {
    params.set('max_results', String(maxResults));
  }

  const url = `/api/youtube/channel/${encodeURIComponent(channelId)}/videos?${params.toString()}`;
  const response = await request(url);

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !validateArray(payload.videos, isYoutubeVideo)) {
    throw new Error('channel videos payload is invalid');
  }

  const videos = payload.videos;
  setCachedChannelVideos(channelId, videos);
  return { fromCache: false, videos };
};

export const refreshYouTubeChannelVideos = async (
  channelId: string,
  maxResults: number | null = null,
): Promise<{ changed: boolean; videos: readonly YoutubeVideo[] }> => {
  const params = new URLSearchParams();
  if (maxResults !== null && maxResults > MIN_RESULTS_THRESHOLD) {
    params.set('max_results', String(maxResults));
  }

  const url = `/api/youtube/channel/${encodeURIComponent(channelId)}/videos?${params.toString()}`;
  const response = await request(url);

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !validateArray(payload.videos, isYoutubeVideo)) {
    throw new Error('channel videos payload is invalid');
  }

  const freshVideos = payload.videos;
  const cached = channelVideosCache.get(channelId);

  if (cached != null && videosAreEqual(cached, freshVideos)) {
    return { changed: false, videos: cached };
  }

  setCachedChannelVideos(channelId, freshVideos);
  return { changed: true, videos: freshVideos };
};

export const getYouTubeEmbedConfig = async (): Promise<YouTubeEmbedConfig> => {
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
    defaults: {
      autoplay: payload.defaults.autoplay,
      quality: payload.defaults.quality,
      quality_dash: payload.defaults.quality_dash,
    },
    invidious_base_url: payload.invidious_base_url,
    referrer_policy: payload.referrer_policy,
  };

  return config;
};

export const getYouTubeThumbnailUrl = (videoId: string): string =>
  `/api/youtube/thumbnail/${encodeURIComponent(videoId)}`;

export const getYouTubePlaylistThumbnailUrl = (playlistId: string): string =>
  `/api/youtube/playlist-thumbnail/${encodeURIComponent(playlistId)}`;

export const getYouTubeVideoMeta = async (videoId: string): Promise<YouTubeVideoMeta> => {
  const response = await request(`/api/youtube/video/${encodeURIComponent(videoId)}/meta`);
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
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

  return {
    duration: payload.video.duration,
    title: payload.video.title,
  };
};

export const getYouTubeVideoProgress = async (videoId: string): Promise<YouTubeWatchProgress> => {
  const response = await request(`/api/youtube/video/${encodeURIComponent(videoId)}/progress`);
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
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
    headers: {
      'content-type': 'application/json',
    },
    method: 'PUT',
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isYouTubeWatchProgress(payload)) {
    throw new Error('youtube watch progress payload is invalid');
  }

  return payload;
};

export const getCachedPlaylistVideos = (playlistId: string): YoutubeVideo[] | null => {
  const cached = playlistVideosCache.get(playlistId);
  return cached ?? null;
};

export const setCachedPlaylistVideos = (
  playlistId: string,
  videos: readonly Readonly<YoutubeVideo>[],
): void => {
  playlistVideosCache.set(playlistId, [...videos]);
};

export const clearPlaylistVideosCache = (playlistId: string | null = null): void => {
  if (playlistId !== null && playlistId !== '') {
    playlistVideosCache.delete(playlistId);
  } else {
    playlistVideosCache.clear();
  }
};

export const getYouTubePlaylists = async (): Promise<readonly YoutubePlaylist[]> => {
  const response = await request('/api/youtube/playlists');
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !validateArray(payload.playlists, isYoutubePlaylist)) {
    throw new Error('playlists payload is invalid');
  }

  return payload.playlists;
};

export const getYouTubePlaylistVideos = async (
  playlistId: string,
): Promise<{ fromCache: boolean; videos: readonly YoutubeVideo[] }> => {
  const cached = playlistVideosCache.get(playlistId);
  if (cached != null) {
    return { fromCache: true, videos: cached };
  }

  const url = `/api/youtube/playlist/${encodeURIComponent(playlistId)}/videos`;
  const response = await request(url);

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !validateArray(payload.videos, isYoutubeVideo)) {
    throw new Error('playlist videos payload is invalid');
  }

  const videos = payload.videos;
  setCachedPlaylistVideos(playlistId, videos);
  return { fromCache: false, videos };
};
