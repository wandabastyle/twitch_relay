import type { ChannelEntry, ChannelStatus, TwitchStatusResponse } from '../api-client/types.js';
import { readJsError } from './errors.js';

const MILLISECONDS_PER_SECOND = 1000;
const MINUTES_PER_CACHE = 5;
const SECONDS_PER_MINUTE = 60;
const EMPTY_ARRAY_LENGTH = 0;

const TWITCH_STATUS_CACHE_KEY = 'twitch_relay:twitch_status';
const TWITCH_STATUS_CACHE_MAX_AGE_MS =
  MILLISECONDS_PER_SECOND * SECONDS_PER_MINUTE * MINUTES_PER_CACHE;

const isValidTwitchStatus = (data: unknown): data is TwitchStatusResponse => {
  if (typeof data !== 'object' || data === null) {
    return false;
  }

  const dataRecord = data as Record<string, unknown>;

  return (
    'connected' in dataRecord &&
    typeof dataRecord.connected === 'boolean' &&
    'scopes' in dataRecord &&
    Array.isArray(dataRecord.scopes)
  );
};

const getFromCache = <T>(key: string, maxAgeMs: number): T | null => {
  try {
    const cached = localStorage.getItem(key);
    if (cached === null) {
      return null;
    }
    const parsed = JSON.parse(cached);
    if (typeof parsed.timestamp !== 'number' || Date.now() - parsed.timestamp > maxAgeMs) {
      return null;
    }
    return parsed.value as T;
  } catch {
    return null;
  }
};

const setCache = (key: string, value: unknown): void => {
  try {
    localStorage.setItem(key, JSON.stringify({ value, timestamp: Date.now() }));
  } catch {
    // Ignore storage errors
  }
};

const clearCache = (key: string): void => {
  try {
    localStorage.removeItem(key);
  } catch {
    // Ignore storage errors
  }
};

export const loadCachedTwitchStatus = (): TwitchStatusResponse | null => {
  const cached = getFromCache<TwitchStatusResponse>(
    TWITCH_STATUS_CACHE_KEY,
    TWITCH_STATUS_CACHE_MAX_AGE_MS,
  );
  if (cached !== null && isValidTwitchStatus(cached)) {
    return cached;
  }
  return null;
};

type ReadonlyTwitchStatus = Readonly<{ connected: boolean; scopes: readonly string[] }>;

export const saveCachedTwitchStatus = (status: ReadonlyTwitchStatus): void => {
  setCache(TWITCH_STATUS_CACHE_KEY, status);
};

export const clearCachedTwitchStatus = (): void => {
  clearCache(TWITCH_STATUS_CACHE_KEY);
};

interface InitialState {
  cachedChannels: ChannelEntry[];
  cachedLiveStatus: Record<string, ChannelStatus>;
  cachedStatus: TwitchStatusResponse | null;
}

export const loadInitialState = (): InitialState => ({
  cachedChannels: [],
  cachedLiveStatus: {},
  cachedStatus: loadCachedTwitchStatus(),
});

export const createInitialTwitchStatus = (
  cachedStatus: TwitchStatusResponse | null,
): TwitchStatusResponse => cachedStatus ?? { connected: false, scopes: [] };

export interface TwitchStatusContext {
  setError: (msg: string | null) => void;
  setLoaded: (value: boolean) => void;
  setStatus: (status: TwitchStatusResponse) => void;
}

export const handleTwitchStatusSuccess = (
  newStatus: TwitchStatusResponse,
  ctx: TwitchStatusContext,
): void => {
  ctx.setStatus(newStatus);
  ctx.setLoaded(true);
  saveCachedTwitchStatus(newStatus);
};

export const handleTwitchStatusError = (
  wasLoaded: boolean,
  ctx: TwitchStatusContext,
  error: unknown,
): void => {
  if (!wasLoaded) {
    ctx.setStatus({ connected: false, scopes: [] });
  }
  ctx.setLoaded(true);
  ctx.setError(readJsError(error, 'failed to load Twitch status'));
};

export interface LiveStatusContext {
  setError: (msg: string | null) => void;
  setLoaded: (value: boolean) => void;
  setStatus: (status: Record<string, ChannelStatus>) => void;
}

export const handleLiveStatusSuccess = (
  channels: Record<string, ChannelStatus>,
  ctx: LiveStatusContext,
): void => {
  ctx.setStatus(channels);
  ctx.setLoaded(true);
  ctx.setError(null);
};

export const handleLiveStatusError = (ctx: LiveStatusContext): void => {
  ctx.setError('Live status refresh is temporarily unavailable');
  ctx.setLoaded(true);
};

export interface ChannelsLoadContext {
  onChannelsLoaded?: () => Promise<void>;
  setChannels: (channels: ChannelEntry[]) => void;
  setError: (msg: string | null) => void;
  setLoaded: (value: boolean) => void;
}

export const handleChannelsLoadSuccess = (
  newChannels: ChannelEntry[],
  ctx: ChannelsLoadContext,
): void => {
  ctx.setChannels(newChannels);
  ctx.setLoaded(true);
  if (ctx.onChannelsLoaded) {
    void ctx.onChannelsLoaded();
  }
};

export const handleChannelsLoadError = (
  currentChannels: ChannelEntry[],
  ctx: ChannelsLoadContext,
  error: unknown,
): void => {
  ctx.setError(readJsError(error, 'failed to load channels'));
  if (currentChannels.length === EMPTY_ARRAY_LENGTH) {
    ctx.setChannels([]);
  }
  ctx.setLoaded(true);
};

export const validateChannelLogin = (login: string): string | null => {
  const normalized = login.trim().toLowerCase();
  return normalized || null;
};

export interface ChannelAddContext {
  addChannelFn: (login: string) => Promise<void>;
  loadChannelsFn: () => Promise<void>;
  setAdding: (value: boolean) => void;
  setError: (msg: string | null) => void;
}

export const executeAddChannel = async (login: string, ctx: ChannelAddContext): Promise<void> => {
  ctx.setAdding(true);
  ctx.setError(null);
  try {
    await ctx.addChannelFn(login);
    await ctx.loadChannelsFn();
  } catch (error) {
    ctx.setError(readJsError(error, 'failed to add channel'));
  } finally {
    ctx.setAdding(false);
  }
};

export interface ChannelRemoveContext {
  loadChannelsFn: () => Promise<void>;
  removeChannelFn: (login: string) => Promise<void>;
  setError: (msg: string | null) => void;
  setRemoving: (value: boolean) => void;
}

export const executeRemoveChannel = async (
  login: string,
  ctx: ChannelRemoveContext,
): Promise<void> => {
  ctx.setRemoving(true);
  ctx.setError(null);
  try {
    await ctx.removeChannelFn(login);
    await ctx.loadChannelsFn();
  } catch (error) {
    ctx.setError(readJsError(error, 'failed to remove channel'));
  } finally {
    ctx.setRemoving(false);
  }
};

export interface WatchContext {
  navigateFn: (url: string) => void;
  setError: (msg: string | null) => void;
  setWatching: (channel: string | null) => void;
}

export const executeStartWatching = async (
  channelLogin: string,
  createTicket: (login: string) => Promise<{ watch_url: string }>,
  ctx: WatchContext,
): Promise<void> => {
  ctx.setWatching(channelLogin);
  ctx.setError(null);
  try {
    const ticket = await createTicket(channelLogin);
    ctx.navigateFn(ticket.watch_url);
  } catch (error) {
    ctx.setError(readJsError(error, `failed to open ${channelLogin}`));
  } finally {
    ctx.setWatching(null);
  }
};

export interface TwitchUnlinkContext {
  disconnectFn: () => Promise<void>;
  loadChannelsFn: () => Promise<void>;
  setBusy: (value: boolean) => void;
  setError: (msg: string | null) => void;
  setStatus: (status: TwitchStatusResponse) => void;
}

export const executeUnlinkTwitch = async (ctx: TwitchUnlinkContext): Promise<void> => {
  ctx.setBusy(true);
  ctx.setError(null);
  try {
    await ctx.disconnectFn();
    ctx.setStatus({ connected: false, scopes: [] });
    clearCachedTwitchStatus();
    await ctx.loadChannelsFn();
  } catch (error) {
    ctx.setError(readJsError(error, 'failed to disconnect Twitch account'));
  } finally {
    ctx.setBusy(false);
  }
};

export interface ControllerMutableState {
  setAddingChannel: (value: boolean) => void;
  setChannels: (channels: ChannelEntry[]) => void;
  setChannelsLoaded: (value: boolean) => void;
  setLiveStatus: (status: Record<string, ChannelStatus>) => void;
  setLiveStatusError: (error: string | null) => void;
  setLiveStatusLoaded: (value: boolean) => void;
  setRemovingChannel: (value: boolean) => void;
  setTwitchBusy: (value: boolean) => void;
  setTwitchStatus: (status: TwitchStatusResponse) => void;
  setTwitchStatusLoaded: (value: boolean) => void;
  setWatchingChannel: (channel: string | null) => void;
}

export const resetControllerState = (ctx: ControllerMutableState): void => {
  ctx.setChannels([]);
  ctx.setLiveStatus({});
  ctx.setLiveStatusError(null);
  ctx.setTwitchStatus({ connected: false, scopes: [] });
  ctx.setTwitchStatusLoaded(false);
  ctx.setChannelsLoaded(false);
  ctx.setLiveStatusLoaded(false);
  ctx.setTwitchBusy(false);
  ctx.setWatchingChannel(null);
  ctx.setAddingChannel(false);
  ctx.setRemovingChannel(false);
};
