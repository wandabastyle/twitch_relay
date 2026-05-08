import {
  addChannel,
  removeChannel,
  getChannels,
  getLiveStatus,
  getTwitchConnectUrl,
  getTwitchStatus,
  disconnectTwitch,
  createWatchTicket,
} from "$lib/api";
import { readMessage } from "$lib/home/errors";
import type { ChannelEntry, ChannelStatus, TwitchStatusResponse } from "$lib/api-client/types";

const TWITCH_STATUS_CACHE_KEY = "twitch_relay:twitch_status";

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

function loadCachedTwitchStatus(): TwitchStatusResponse | null {
  try {
    const cached = sessionStorage.getItem(TWITCH_STATUS_CACHE_KEY);
    if (cached) {
      const parsed = JSON.parse(cached) as TwitchStatusResponse;
      // Basic validation to ensure cached data has expected shape
      if (typeof parsed.connected === "boolean" && Array.isArray(parsed.scopes)) {
        return parsed;
      }
    }
  } catch {
    // Ignore sessionStorage errors (e.g., private browsing mode)
  }
  return null;
}

function saveCachedTwitchStatus(status: TwitchStatusResponse): void {
  try {
    sessionStorage.setItem(TWITCH_STATUS_CACHE_KEY, JSON.stringify(status));
  } catch {
    // Ignore sessionStorage errors
  }
}

function clearCachedTwitchStatus(): void {
  try {
    sessionStorage.removeItem(TWITCH_STATUS_CACHE_KEY);
  } catch {
    // Ignore sessionStorage errors
  }
}

export function createChannelsController(deps: ChannelsControllerDeps): ChannelsController {
  // Try to load cached status immediately to prevent UI flash
  const cachedStatus = loadCachedTwitchStatus();
  const initialStatus: TwitchStatusResponse = cachedStatus ?? { connected: false, scopes: [] };

  let channels = $state<Array<ChannelEntry>>([]);
  let liveStatus = $state<Record<string, ChannelStatus>>({});
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
      setError(readMessage(err, "failed to load Twitch status"));
    }
  }

  async function loadChannels(): Promise<void> {
    try {
      channels = await getChannels();
      await loadLiveStatus();
      await loadTwitchStatus();
      if (onChannelsLoaded) {
        await onChannelsLoaded();
      }
    } catch (err) {
      setError(readMessage(err, "failed to load channels"));
      channels = [];
    }
  }

  async function loadLiveStatus(): Promise<void> {
    try {
      const status = await getLiveStatus();
      liveStatus = status.channels;
      liveStatusError = null;
    } catch {
      liveStatusError = "Live status refresh is temporarily unavailable";
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
      setError(readMessage(err, "failed to add channel"));
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
      setError(readMessage(err, "failed to remove channel"));
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
      setError(readMessage(err, "failed to disconnect Twitch account"));
    } finally {
      isTwitchBusy = false;
    }
  }

  async function startWatching(channelLogin: string): Promise<void> {
    watchingChannel = channelLogin;
    setError(null);

    try {
      const ticket = await createWatchTicket(channelLogin);
      window.location.assign(ticket.watch_url);
    } catch (err) {
      setError(readMessage(err, `failed to open ${channelLogin}`));
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
