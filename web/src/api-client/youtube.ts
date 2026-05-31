import { isObject, readApiError, request, safeJson } from './core.js';
import type { YoutubeChannel, YoutubeChannelInfo, YoutubePlaylist, YoutubeVideo } from './types.js';

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

const validateArray = <ItemType>(
  value: unknown,
  predicate: (item: unknown) => item is ItemType,
): value is readonly ItemType[] =>
  Array.isArray(value) && value.every((item: unknown) => predicate(item));

const parseVideosPayload = (payload: unknown, errorMessage: string): readonly YoutubeVideo[] => {
  if (!isObject(payload) || !validateArray(payload.videos, isYoutubeVideo)) {
    throw new Error(errorMessage);
  }
  return payload.videos;
};

const createMaxResultsParams = (maxResults: number | null): URLSearchParams => {
  const params = new URLSearchParams();
  if (maxResults !== null && maxResults > MIN_RESULTS_THRESHOLD) {
    params.set('max_results', String(maxResults));
  }
  return params;
};

const createChannelVideosUrl = (channelId: string, maxResults: number | null): string =>
  `/api/youtube/channel/${encodeURIComponent(channelId)}/videos?${createMaxResultsParams(maxResults).toString()}`;

const fetchChannelVideos = async (
  channelId: string,
  maxResults: number | null,
): Promise<readonly YoutubeVideo[]> => {
  const response = await request(createChannelVideosUrl(channelId, maxResults));
  if (!response.ok) {
    throw new Error(readApiError(await safeJson(response)));
  }
  return parseVideosPayload(await safeJson(response), 'channel videos payload is invalid');
};

const fetchPlaylistVideos = async (playlistId: string): Promise<readonly YoutubeVideo[]> => {
  const response = await request(`/api/youtube/playlist/${encodeURIComponent(playlistId)}/videos`);
  if (!response.ok) {
    throw new Error(readApiError(await safeJson(response)));
  }
  return parseVideosPayload(await safeJson(response), 'playlist videos payload is invalid');
};

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

    const INCREMENT_STEP = 1;
    for (let index = 0; index < videosA.length; index += INCREMENT_STEP) {
    const videoA = videosA[index];
    const videoB = videosB[index];
    if (
      videoA.video_id !== videoB.video_id ||
      videoA.title !== videoB.title ||
      videoA.duration !== videoB.duration ||
      videoA.view_count !== videoB.view_count ||
      videoA.thumbnail !== videoB.thumbnail
    ) {
      return false;
    }
  }

  return true;
};

export const getYouTubeChannelVideos = async (
  channelId: string,
  maxResults: number | null = null,
): Promise<{ fromCache: boolean; videos: readonly YoutubeVideo[] }> => {
  const cached = channelVideosCache.get(channelId);
  if (cached !== undefined) {
    return { fromCache: true, videos: cached };
  }
  const videos = await fetchChannelVideos(channelId, maxResults);
  setCachedChannelVideos(channelId, videos);
  return { fromCache: false, videos };
};

export const refreshYouTubeChannelVideos = async (
  channelId: string,
  maxResults: number | null = null,
): Promise<{ changed: boolean; videos: readonly YoutubeVideo[] }> => {
  const freshVideos = await fetchChannelVideos(channelId, maxResults);
  const cached = channelVideosCache.get(channelId);
  if (cached !== undefined && videosAreEqual(cached, freshVideos)) {
    return { changed: false, videos: cached };
  }

  setCachedChannelVideos(channelId, freshVideos);
  return { changed: true, videos: freshVideos };
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
  if (cached !== undefined) {
    return { fromCache: true, videos: cached };
  }
  const videos = await fetchPlaylistVideos(playlistId);
  setCachedPlaylistVideos(playlistId, videos);
  return { fromCache: false, videos };
};

export const refreshYouTubePlaylistVideos = async (
  playlistId: string,
): Promise<{ changed: boolean; videos: readonly YoutubeVideo[] }> => {
  const freshVideos = await fetchPlaylistVideos(playlistId);
  const cached = playlistVideosCache.get(playlistId);
  if (cached !== undefined && videosAreEqual(cached, freshVideos)) {
    return { changed: false, videos: cached };
  }

  setCachedPlaylistVideos(playlistId, freshVideos);
  return { changed: true, videos: freshVideos };
};
