import type {
  YouTubeEmbedConfig,
  YouTubeVideoMeta,
  YouTubeWatchProgress,
  YoutubeChannel,
  YoutubeChannelInfo,
  YoutubePlaylist,
  YoutubeVideo,
} from './types.js';
import { isObject, readApiError, request, safeJson } from './core.js';

const DEFAULT_MAX_RESULTS = 25;

const channelVideosCache = new Map<string, YoutubeVideo[]>();
const playlistVideosCache = new Map<string, YoutubeVideo[]>();

export const getCachedChannelVideos = (channelId: string): YoutubeVideo[] | undefined =>
  channelVideosCache.get(channelId);

export const setCachedChannelVideos = (channelId: string, videos: YoutubeVideo[]): void => {
  channelVideosCache.set(channelId, videos);
};

export const clearChannelVideosCache = (channelId?: string): void => {
  if (channelId !== undefined && channelId !== '') {
    channelVideosCache.delete(channelId);
  } else {
    channelVideosCache.clear();
  }
};

export const getYouTubeSubscriptions = async (): Promise<YoutubeChannel[]> => {
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
};

export const getYouTubeRecentVideos = async (
  maxResults = DEFAULT_MAX_RESULTS,
): Promise<YoutubeVideo[]> => {
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
};

export const getYouTubeChannelInfo = async (channelId: string): Promise<YoutubeChannelInfo> => {
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
};

const videosAreEqual = (a: YoutubeVideo[], b: YoutubeVideo[]): boolean => {
  if (a.length !== b.length) {
    return false;
  }
  const aIds = a.map((video) => video.video_id);
  const bIds = b.map((video) => video.video_id);
  return JSON.stringify(aIds) === JSON.stringify(bIds);
};

export const getYouTubeChannelVideos = async (
  channelId: string,
  maxResults?: number,
): Promise<{ fromCache: boolean; videos: YoutubeVideo[] }> => {
  const cached = channelVideosCache.get(channelId);
  if (cached !== undefined) {
    return { fromCache: true, videos: cached };
  }

  const params = new URLSearchParams();
  if (maxResults !== undefined && maxResults !== 0) {
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
  return { fromCache: false, videos };
};

export const refreshYouTubeChannelVideos = async (
  channelId: string,
  maxResults?: number,
): Promise<{ changed: boolean; videos: YoutubeVideo[] }> => {
  const params = new URLSearchParams();
  if (maxResults !== undefined && maxResults !== 0) {
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

  if (cached !== undefined && videosAreEqual(cached, freshVideos)) {
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
};

export const saveYouTubeVideoProgress = async (
  videoId: string,
  progress: {
    completed?: boolean;
    duration_secs?: number | null;
    position_secs: number;
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
};

export const getCachedPlaylistVideos = (playlistId: string): YoutubeVideo[] | undefined =>
  playlistVideosCache.get(playlistId);

export const setCachedPlaylistVideos = (playlistId: string, videos: YoutubeVideo[]): void => {
  playlistVideosCache.set(playlistId, videos);
};

export const clearPlaylistVideosCache = (playlistId?: string): void => {
  if (playlistId !== undefined && playlistId !== '') {
    playlistVideosCache.delete(playlistId);
  } else {
    playlistVideosCache.clear();
  }
};

export const getYouTubePlaylists = async (): Promise<YoutubePlaylist[]> => {
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
};

export const getYouTubePlaylistVideos = async (
  playlistId: string,
): Promise<{ fromCache: boolean; videos: YoutubeVideo[] }> => {
  const cached = playlistVideosCache.get(playlistId);
  if (cached !== undefined) {
    return { fromCache: true, videos: cached };
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
  return { fromCache: false, videos };
};
