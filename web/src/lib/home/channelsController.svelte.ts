import {
  addChannel,
  removeChannel,
  getChannels,
  getLiveStatus,
  getTwitchConnectUrl,
  getTwitchStatus,
  disconnectTwitch,
  createWatchTicket,
  getCachedChannels,
  getCachedLiveStatus,
} from "$lib/api-client";
import { goto } from "$app/navigation";
import { readJsError } from "$lib/home/errors";
import { getFromCache, setCache, clearCache } from "$lib/cache";
import type { ChannelEntry, ChannelStatus, TwitchStatusResponse } from "$lib/api-client/types";

const TWITCH_STATUS_CACHE_KEY = "twitch_relay:twitch_status";
const TWITCH_STATUS_CACHE_MAX_AGE_MS = 5 * 60 * 1000; // 5 minutes

export interface ChannelsControllerDeps {
  setError: (message: string | null) => void;
  onChannelsLoaded?: () => Promise<void>;
}

export interface ChannelsController {
  channels: Array<ChannelEntry>;
  liveStatus: Record<string, ChannelStatus>;
  liveStatusError: string | null;
  twitchStatus: TwitchStatusResponse;
  isTwitchStatusLoaded: boolean;
  isChannelsLoaded: boolean;
  isLiveStatusLoaded: boolean;
  isTwitchBusy: boolean;
  watchingChannel: string | null;
  isAddingChannel: boolean;
  isRemovingChannel: boolean;

  loadChannels: () => Promise<void>;
  loadLiveStatus: () => Promise<void>;
  submitAddChannel: (newChannelLogin: string) => Promise<void>;
  confirmRemoveChannel: (login: string) => Promise<void>;
  connectTwitch: () => void;
  unlinkTwitch: () => Promise<void>;
  startWatching: (channelLogin: string) => Promise<void>;
  resetState: () => void;
}

function isValidTwitchStatus(data: unknown): data is TwitchStatusResponse {
  return (
    typeof data === "object" &&
    data !== null &&
    "connected" in data &&
    typeof (data as TwitchStatusResponse).connected === "boolean" &&
    "scopes" in data &&
    Array.isArray((data as TwitchStatusResponse).scopes)
  );
}

function loadCachedTwitchStatus(): TwitchStatusResponse | null {
  const cached = getFromCache<TwitchStatusResponse>(
    TWITCH_STATUS_CACHE_KEY,
    TWITCH_STATUS_CACHE_MAX_AGE_MS,
  );
  if (cached && isValidTwitchStatus(cached)) {
    return cached;
  }
  return null;
}

function saveCachedTwitchStatus(status: TwitchStatusResponse): void {
  setCache(TWITCH_STATUS_CACHE_KEY, status);
}

function clearCachedTwitchStatus(): void {
  clearCache(TWITCH_STATUS_CACHE_KEY);
}

export function createChannelsController(deps: ChannelsControllerDeps): ChannelsController {
  // Try to load cached status, channels, and live status immediately to prevent UI flash
  const cachedStatus = loadCachedTwitchStatus();
  const initialStatus: TwitchStatusResponse = cachedStatus ?? { connected: false, scopes: [] };
  const cachedChannels = getCachedChannels();
  const cachedLiveStatus = getCachedLiveStatus();

  let channels = $state<Array<ChannelEntry>>(cachedChannels);
  let isChannelsLoaded = $state<boolean>(cachedChannels.length > 0);
  let liveStatus = $state<Record<string, ChannelStatus>>(cachedLiveStatus);
  let isLiveStatusLoaded = $state<boolean>(Object.keys(cachedLiveStatus).length > 0);
  let liveStatusError = $state<string | null>(null);
  let twitchStatus = $state<TwitchStatusResponse>(initialStatus);
  // If we have cached data, consider it "loaded" initially to avoid showing loading state
  let isTwitchStatusLoaded = $state<boolean>(cachedStatus !== null);
  let isTwitchBusy = $state(false);
  let watchingChannel = $state<string | null>(null);
  let isAddingChannel = $state(false);
  let isRemovingChannel = $state(false);

  const { setError, onChannelsLoaded } = deps;

  async function loadTwitchStatus(): Promise<void> {
    try {
      const newStatus = await getTwitchStatus();
      twitchStatus = newStatus;
      isTwitchStatusLoaded = true;
      // Cache the successful response
      saveCachedTwitchStatus(newStatus);
    } catch (err) {
      // Conservative failure handling: only update if we don't have a cached value
      // If API fails but we have cached data, keep showing cached state
      if (!isTwitchStatusLoaded) {
        twitchStatus = { connected: false, scopes: [] };
      }
      // Mark as loaded even on error so UI doesn't stay in loading state indefinitely
      isTwitchStatusLoaded = true;
      setError(readJsError(err, "failed to load Twitch status"));
    }
  }

  async function loadChannels(): Promise<void> {
    const twitchStatusPromise = loadTwitchStatus();

    try {
      channels = await getChannels();
      isChannelsLoaded = true;
      await loadLiveStatus();
      if (onChannelsLoaded) {
        await onChannelsLoaded();
      }
    } catch (err) {
      setError(readJsError(err, "failed to load channels"));
      // If fetch fails but we have cached channels, keep them
      if (channels.length === 0) {
        channels = [];
      }
      isChannelsLoaded = true;
    }

    await twitchStatusPromise;
  }

  async function loadLiveStatus(): Promise<void> {
    try {
      const status = await getLiveStatus();
      liveStatus = status.channels;
      isLiveStatusLoaded = true;
      liveStatusError = null;
    } catch {
      liveStatusError = "Live status refresh is temporarily unavailable";
      // If we have cached live status, keep it even if refresh fails
      isLiveStatusLoaded = true;
    }
  }

  async function submitAddChannel(newChannelLogin: string): Promise<void> {
    const normalized = newChannelLogin.trim().toLowerCase();
    if (!normalized) {
      setError("channel name is required");
      return;
    }

    isAddingChannel = true;
    setError(null);

    try {
      await addChannel(normalized);
      await loadChannels();
    } catch (err) {
      setError(readJsError(err, "failed to add channel"));
    } finally {
      isAddingChannel = false;
    }
  }

  async function confirmRemoveChannel(login: string): Promise<void> {
    isRemovingChannel = true;
    setError(null);

    try {
      await removeChannel(login);
      await loadChannels();
    } catch (err) {
      setError(readJsError(err, "failed to remove channel"));
    } finally {
      isRemovingChannel = false;
    }
  }

  function connectTwitch(): void {
    window.location.assign(getTwitchConnectUrl());
  }

  async function unlinkTwitch(): Promise<void> {
    isTwitchBusy = true;
    setError(null);
    try {
      await disconnectTwitch();
      twitchStatus = { connected: false, scopes: [] };
      // Clear cache when explicitly disconnected
      clearCachedTwitchStatus();
      await loadChannels();
    } catch (err) {
      setError(readJsError(err, "failed to disconnect Twitch account"));
    } finally {
      isTwitchBusy = false;
    }
  }

  async function startWatching(channelLogin: string): Promise<void> {
    watchingChannel = channelLogin;
    setError(null);

    try {
      const ticket = await createWatchTicket(channelLogin);
      await goto(ticket.watch_url);
    } catch (err) {
      setError(readJsError(err, `failed to open ${channelLogin}`));
    } finally {
      watchingChannel = null;
    }
  }

  function resetState(): void {
    channels = [];
    liveStatus = {};
    liveStatusError = null;
    twitchStatus = { connected: false, scopes: [] };
    isTwitchStatusLoaded = false;
    isChannelsLoaded = false;
    isLiveStatusLoaded = false;
    isTwitchBusy = false;
    watchingChannel = null;
    isAddingChannel = false;
    isRemovingChannel = false;
    clearCachedTwitchStatus();
  }

  return {
    get channels() {
      return channels;
    },
    get liveStatus() {
      return liveStatus;
    },
    get liveStatusError() {
      return liveStatusError;
    },
    get twitchStatus() {
      return twitchStatus;
    },
    get isTwitchStatusLoaded() {
      return isTwitchStatusLoaded;
    },
    get isChannelsLoaded() {
      return isChannelsLoaded;
    },
    get isLiveStatusLoaded() {
      return isLiveStatusLoaded;
    },
    get isTwitchBusy() {
      return isTwitchBusy;
    },
    get watchingChannel() {
      return watchingChannel;
    },
    get isAddingChannel() {
      return isAddingChannel;
    },
    get isRemovingChannel() {
      return isRemovingChannel;
    },
    loadChannels,
    loadLiveStatus,
    submitAddChannel,
    confirmRemoveChannel,
    connectTwitch,
    unlinkTwitch,
    startWatching,
    resetState,
  };
}
