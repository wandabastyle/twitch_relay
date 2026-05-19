import { clearCache, getFromCache, setCache } from '$lib/cache';

import { isObject, readApiError, request, safeJson } from './core.js';
import type {
  ChannelEntry,
  ChannelStatus,
  LiveStatusResponse,
  WatchTicketResponse,
} from './types.js';

const CACHE_AGE_ONE_MINUTE_MS = 60_000;
const CACHE_AGE_FIVE_MINUTES_MS = 300_000;

const CHANNELS_CACHE_KEY = 'twitchRelay.channels';
const CHANNELS_CACHE_MAX_AGE_MS = CACHE_AGE_FIVE_MINUTES_MS;

const LIVE_STATUS_CACHE_KEY = 'twitchRelay.liveStatus';
const LIVE_STATUS_CACHE_MAX_AGE_MS = CACHE_AGE_ONE_MINUTE_MS;

const getChannelsFromCache = (): ChannelEntry[] | undefined => {
  const cached = getFromCache<ChannelEntry[]>(CHANNELS_CACHE_KEY, CHANNELS_CACHE_MAX_AGE_MS);
  if (!cached) {
    return undefined;
  }

  // Validate cache entries
  return cached.filter(
    (item): item is ChannelEntry =>
      isObject(item) &&
      typeof item.login === 'string' &&
      (item.source === 'manual' || item.source === 'followed' || item.source === 'both') &&
      typeof item.removable === 'boolean',
  );
};

const setChannelsCache = (data: readonly ChannelEntry[]): void => {
  setCache(CHANNELS_CACHE_KEY, data);
};

export const clearChannelsCache = (): void => {
  clearCache(CHANNELS_CACHE_KEY);
};

export const getChannels = async (): Promise<ChannelEntry[]> => {
  const response = await request('/api/channels');
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !Array.isArray(payload.channels)) {
    throw new Error('channels payload is invalid');
  }

  const channels = payload.channels.filter(
    (item): item is ChannelEntry =>
      isObject(item) &&
      typeof item.login === 'string' &&
      (item.source === 'manual' || item.source === 'followed' || item.source === 'both') &&
      typeof item.removable === 'boolean',
  );

  // Cache successful response
  setChannelsCache(channels);

  return channels;
};

export const getCachedChannels = (): ChannelEntry[] => {
  const cached = getChannelsFromCache();
  if (cached === undefined) {
    return [];
  }
  return cached;
};

export const getCachedLiveStatus = (): Record<string, ChannelStatus> => {
  const cached = getLiveStatusFromCache();
  if (cached === undefined) {
    return {};
  }
  return cached.channels;
};

export const createWatchTicket = async (channelLogin: string): Promise<WatchTicketResponse> => {
  const response = await request('/api/watch-ticket', {
    body: JSON.stringify({ channel_login: channelLogin }),
    headers: {
      'content-type': 'application/json',
    },
    method: 'POST',
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.watch_url !== 'string') {
    throw new Error('watch ticket payload is invalid');
  }

  return {
    watch_url: payload.watch_url,
  };
};

export const addChannel = async (login: string): Promise<void> => {
  const response = await request('/api/channels', {
    body: JSON.stringify({ login }),
    headers: {
      'content-type': 'application/json',
    },
    method: 'POST',
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  // Clear cache so fresh data is fetched on next load
  clearChannelsCache();
};

export const removeChannel = async (login: string): Promise<void> => {
  const response = await request(`/api/channels/${encodeURIComponent(login)}`, {
    method: 'DELETE',
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  // Clear cache so fresh data is fetched on next load
  clearChannelsCache();
};

const isChannelStatus = (value: unknown): value is ChannelStatus =>
  isObject(value) && typeof value.is_live === 'boolean' && typeof value.viewer_count === 'number';

const parseLiveStatusPayload = (payload: unknown): LiveStatusResponse => {
  if (!isObject(payload) || !isObject(payload.channels)) {
    throw new Error('live status payload is invalid');
  }

  const channels: Record<string, ChannelStatus> = {};
  for (const [key, value] of Object.entries(payload.channels)) {
    if (isChannelStatus(value)) {
      channels[key] = value;
    }
  }

  return {
    channels,
  };
};

const getLiveStatusFromCache = (): LiveStatusResponse | undefined => {
  const cached = getFromCache<LiveStatusResponse>(
    LIVE_STATUS_CACHE_KEY,
    LIVE_STATUS_CACHE_MAX_AGE_MS,
  );
  if (!cached) {
    return undefined;
  }

  try {
    return parseLiveStatusPayload(cached);
  } catch {
    return undefined;
  }
};

const setLiveStatusCache = (data: Readonly<LiveStatusResponse>): void => {
  setCache(LIVE_STATUS_CACHE_KEY, data);
};

const fetchLiveStatusFromApi = async (): Promise<LiveStatusResponse> => {
  const response = await request('/api/live-status');
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  return parseLiveStatusPayload(payload);
};

const refreshLiveStatusCache = async (): Promise<void> => {
  try {
    const fresh = await fetchLiveStatusFromApi();
    setLiveStatusCache(fresh);
  } catch {
    // Keep existing cache if refresh fails.
  }
};

export const getLiveStatus = async (): Promise<LiveStatusResponse> => {
  const cached = getLiveStatusFromCache();
  if (cached) {
    void refreshLiveStatusCache();
    return cached;
  }

  const fresh = await fetchLiveStatusFromApi();
  setLiveStatusCache(fresh);
  return fresh;
};
