import { clearCache, getFromCache, setCache } from '$lib/cache';
import { navigate } from '$lib/router/router.svelte';
import {
  addChannel,
  createWatchTicket,
  disconnectTwitch,
  getCachedChannels,
  getCachedLiveStatus,
  getChannels,
  getLiveStatus,
  getTwitchConnectUrl,
  getTwitchStatus,
  removeChannel,
} from '$lib/api-client';
import { readJsError } from '$lib/home/errors';
import type { ChannelEntry, ChannelStatus, TwitchStatusResponse } from '$lib/api-client/types';

const MILLISECONDS_PER_SECOND = 1_000;
const MINUTES_PER_CACHE = 5;
const SECONDS_PER_MINUTE = 60;

const TWITCH_STATUS_CACHE_KEY = 'twitch_relay:twitch_status';
const TWITCH_STATUS_CACHE_MAX_AGE_MS =
  MILLISECONDS_PER_SECOND * SECONDS_PER_MINUTE * MINUTES_PER_CACHE;

export interface ChannelsControllerDeps {
  onChannelsLoaded?: () => Promise<void>;
  setError: (message: string | undefined) => void;
}

export interface ChannelsController {
  channels: ChannelEntry[];
  confirmRemoveChannel: (login: string) => Promise<void>;
  connectTwitch: () => void;
  isAddingChannel: boolean;
  isChannelsLoaded: boolean;
  isLiveStatusLoaded: boolean;
  isRemovingChannel: boolean;
  isTwitchBusy: boolean;
  isTwitchStatusLoaded: boolean;
  liveStatus: Record<string, ChannelStatus>;
  liveStatusError: string | undefined;
  loadChannels: () => Promise<void>;
  loadLiveStatus: () => Promise<void>;
  resetState: () => void;
  startWatching: (channelLogin: string) => Promise<void>;
  submitAddChannel: (newChannelLogin: string) => Promise<void>;
  twitchStatus: TwitchStatusResponse;
  unlinkTwitch: () => Promise<void>;
  watchingChannel: string | undefined;
}

const isValidTwitchStatus = (data: unknown): data is TwitchStatusResponse => {
  return (
    typeof data === 'object' &&
    data !== null &&
    'connected' in data &&
    typeof (data as TwitchStatusResponse).connected === 'boolean' &&
    'scopes' in data &&
    Array.isArray((data as TwitchStatusResponse).scopes)
  );
};

const loadCachedTwitchStatus = (): TwitchStatusResponse | undefined => {
  const cached = getFromCache<TwitchStatusResponse>(
    TWITCH_STATUS_CACHE_KEY,
    TWITCH_STATUS_CACHE_MAX_AGE_MS,
  );
  if (cached && isValidTwitchStatus(cached)) {
    return cached;
  }
  return undefined;
};

const saveCachedTwitchStatus = (status: TwitchStatusResponse): void => {
  setCache(TWITCH_STATUS_CACHE_KEY, status);
};

const clearCachedTwitchStatus = (): void => {
  clearCache(TWITCH_STATUS_CACHE_KEY);
};

export const createChannelsController = (deps: ChannelsControllerDeps): ChannelsController => {
  // Try to load cached status, channels, and live status immediately to prevent UI flash
  const cachedStatus = loadCachedTwitchStatus();
  const initialStatus: TwitchStatusResponse = cachedStatus
    ? cachedStatus
    : { connected: false, scopes: [] };
  const cachedChannels = getCachedChannels();
  const cachedLiveStatus = getCachedLiveStatus();

  let channels = $state<ChannelEntry[]>(cachedChannels);
  let isChannelsLoaded = $state<boolean>(cachedChannels.length > 0);
  let liveStatus = $state<Record<string, ChannelStatus>>(cachedLiveStatus);
  let isLiveStatusLoaded = $state<boolean>(Object.keys(cachedLiveStatus).length > 0);
  let liveStatusError = $state<string | undefined>(undefined);
  let twitchStatus = $state<TwitchStatusResponse>(initialStatus);
  // If we have cached data, consider it "loaded" initially to avoid showing loading state
  let isTwitchStatusLoaded = $state<boolean>(cachedStatus !== undefined);
  let isTwitchBusy = $state(false);
  let watchingChannel = $state<string | undefined>(undefined);
  let isAddingChannel = $state(false);
  let isRemovingChannel = $state(false);

  const { onChannelsLoaded, setError } = deps;

  const loadTwitchStatus = async (): Promise<void> => {
    try {
      const newStatus = await getTwitchStatus();
      twitchStatus = newStatus;
      isTwitchStatusLoaded = true;
      // Cache the successful response
      saveCachedTwitchStatus(newStatus);
    } catch (error) {
      // Conservative failure handling: only update if we don't have a cached value
      // If API fails but we have cached data, keep showing cached state
      if (!isTwitchStatusLoaded) {
        twitchStatus = { connected: false, scopes: [] };
      }
      // Mark as loaded even on error so UI doesn't stay in loading state indefinitely
      isTwitchStatusLoaded = true;
      setError(readJsError(error, 'failed to load Twitch status'));
    }
  };

  const loadChannels = async (): Promise<void> => {
    const twitchStatusPromise = loadTwitchStatus();

    try {
      channels = await getChannels();
      isChannelsLoaded = true;
      await loadLiveStatus();
      if (onChannelsLoaded) {
        await onChannelsLoaded();
      }
    } catch (error) {
      setError(readJsError(error, 'failed to load channels'));
      // If fetch fails but we have cached channels, keep them
      if (channels.length === 0) {
        channels = [];
      }
      isChannelsLoaded = true;
    }

    await twitchStatusPromise;
  };

  const loadLiveStatus = async (): Promise<void> => {
    try {
      const status = await getLiveStatus();
      liveStatus = status.channels;
      isLiveStatusLoaded = true;
      liveStatusError = undefined;
    } catch {
      liveStatusError = 'Live status refresh is temporarily unavailable';
      // If we have cached live status, keep it even if refresh fails
      isLiveStatusLoaded = true;
    }
  };

  const submitAddChannel = async (newChannelLogin: string): Promise<void> => {
    const normalized = newChannelLogin.trim().toLowerCase();
    if (!normalized) {
      setError('channel name is required');
      return;
    }

    isAddingChannel = true;
    setError(undefined);

    try {
      await addChannel(normalized);
      await loadChannels();
    } catch (error) {
      setError(readJsError(error, 'failed to add channel'));
    } finally {
      isAddingChannel = false;
    }
  };

  const confirmRemoveChannel = async (login: string): Promise<void> => {
    isRemovingChannel = true;
    setError(undefined);

    try {
      await removeChannel(login);
      await loadChannels();
    } catch (error) {
      setError(readJsError(error, 'failed to remove channel'));
    } finally {
      isRemovingChannel = false;
    }
  };

  const connectTwitch = (): void => {
    globalThis.window.location.assign(getTwitchConnectUrl());
  };

  const unlinkTwitch = async (): Promise<void> => {
    isTwitchBusy = true;
    setError(undefined);
    try {
      await disconnectTwitch();
      twitchStatus = { connected: false, scopes: [] };
      // Clear cache when explicitly disconnected
      clearCachedTwitchStatus();
      await loadChannels();
    } catch (error) {
      setError(readJsError(error, 'failed to disconnect Twitch account'));
    } finally {
      isTwitchBusy = false;
    }
  };

  const startWatching = async (channelLogin: string): Promise<void> => {
    watchingChannel = channelLogin;
    setError(undefined);

    try {
      const ticket = await createWatchTicket(channelLogin);
      navigate(ticket.watch_url);
    } catch (error) {
      setError(readJsError(error, `failed to open ${channelLogin}`));
    } finally {
      watchingChannel = undefined;
    }
  };

  const resetState = (): void => {
    channels = [];
    liveStatus = {};
    liveStatusError = undefined;
    twitchStatus = { connected: false, scopes: [] };
    isTwitchStatusLoaded = false;
    isChannelsLoaded = false;
    isLiveStatusLoaded = false;
    isTwitchBusy = false;
    watchingChannel = undefined;
    isAddingChannel = false;
    isRemovingChannel = false;
    clearCachedTwitchStatus();
  };

  return {
    get channels() {
      return channels;
    },
    confirmRemoveChannel,
    connectTwitch,
    get isAddingChannel() {
      return isAddingChannel;
    },
    get isChannelsLoaded() {
      return isChannelsLoaded;
    },
    get isLiveStatusLoaded() {
      return isLiveStatusLoaded;
    },
    get isRemovingChannel() {
      return isRemovingChannel;
    },
    get isTwitchBusy() {
      return isTwitchBusy;
    },
    get isTwitchStatusLoaded() {
      return isTwitchStatusLoaded;
    },
    get liveStatus() {
      return liveStatus;
    },
    get liveStatusError() {
      return liveStatusError;
    },
    loadChannels,
    loadLiveStatus,
    resetState,
    startWatching,
    submitAddChannel,
    get twitchStatus() {
      return twitchStatus;
    },
    unlinkTwitch,
    get watchingChannel() {
      return watchingChannel;
    },
  };
};
