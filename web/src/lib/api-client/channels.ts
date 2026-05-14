import { isObject, safeJson, readApiError, request } from "./core";
import type { ChannelEntry, LiveStatusResponse, ChannelStatus, WatchTicketResponse } from "./types";
import { getFromCache, setCache, clearCache } from "$lib/cache";

const LIVE_STATUS_CACHE_KEY = "twitchRelay.liveStatus";
const LIVE_STATUS_CACHE_MAX_AGE_MS = 60000;

const CHANNELS_CACHE_KEY = "twitchRelay.channels";
const CHANNELS_CACHE_MAX_AGE_MS = 5 * 60 * 1000; // 5 minutes

function getChannelsFromCache(): Array<ChannelEntry> | null {
  const cached = getFromCache<Array<ChannelEntry>>(CHANNELS_CACHE_KEY, CHANNELS_CACHE_MAX_AGE_MS);
  if (!cached) return null;

  // Validate cache entries
  return cached.filter(
    (item): item is ChannelEntry =>
      isObject(item) &&
      typeof item.login === "string" &&
      (item.source === "manual" || item.source === "followed" || item.source === "both") &&
      typeof item.removable === "boolean",
  );
}

function setChannelsCache(data: Array<ChannelEntry>): void {
  setCache(CHANNELS_CACHE_KEY, data);
}

export function clearChannelsCache(): void {
  clearCache(CHANNELS_CACHE_KEY);
}

export async function getChannels(): Promise<Array<ChannelEntry>> {
  const response = await request("/api/channels");
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !Array.isArray(payload.channels)) {
    throw new Error("channels payload is invalid");
  }

  const channels = payload.channels.filter(
    (item): item is ChannelEntry =>
      isObject(item) &&
      typeof item.login === "string" &&
      (item.source === "manual" || item.source === "followed" || item.source === "both") &&
      typeof item.removable === "boolean",
  );

  // Cache successful response
  setChannelsCache(channels);

  return channels;
}

export function getCachedChannels(): Array<ChannelEntry> {
  return getChannelsFromCache() ?? [];
}

export function getCachedLiveStatus(): Record<string, ChannelStatus> {
  const cached = getLiveStatusFromCache();
  return cached?.channels ?? {};
}

export async function createWatchTicket(channelLogin: string): Promise<WatchTicketResponse> {
  const response = await request("/api/watch-ticket", {
    method: "POST",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify({ channel_login: channelLogin }),
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.watch_url !== "string") {
    throw new Error("watch ticket payload is invalid");
  }

  return {
    watch_url: payload.watch_url,
  };
}

export async function addChannel(login: string): Promise<void> {
  const response = await request("/api/channels", {
    method: "POST",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify({ login }),
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  // Clear cache so fresh data is fetched on next load
  clearChannelsCache();
}

export async function removeChannel(login: string): Promise<void> {
  const response = await request(`/api/channels/${encodeURIComponent(login)}`, {
    method: "DELETE",
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  // Clear cache so fresh data is fetched on next load
  clearChannelsCache();
}

function parseLiveStatusPayload(payload: unknown): LiveStatusResponse {
  if (!isObject(payload) || !isObject(payload.channels)) {
    throw new Error("live status payload is invalid");
  }

  return {
    channels: payload.channels as Record<string, ChannelStatus>,
  };
}

function getLiveStatusFromCache(): LiveStatusResponse | null {
  const cached = getFromCache<LiveStatusResponse>(
    LIVE_STATUS_CACHE_KEY,
    LIVE_STATUS_CACHE_MAX_AGE_MS,
  );
  if (!cached) return null;

  try {
    return parseLiveStatusPayload(cached);
  } catch {
    return null;
  }
}

function setLiveStatusCache(data: LiveStatusResponse): void {
  setCache(LIVE_STATUS_CACHE_KEY, data);
}

async function fetchLiveStatusFromApi(): Promise<LiveStatusResponse> {
  const response = await request("/api/live-status");
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  return parseLiveStatusPayload(payload);
}

async function refreshLiveStatusCache(): Promise<void> {
  try {
    const fresh = await fetchLiveStatusFromApi();
    setLiveStatusCache(fresh);
  } catch {
    // Keep existing cache if refresh fails.
  }
}

export async function getLiveStatus(): Promise<LiveStatusResponse> {
  const cached = getLiveStatusFromCache();
  if (cached) {
    void refreshLiveStatusCache();
    return cached;
  }

  const fresh = await fetchLiveStatusFromApi();
  setLiveStatusCache(fresh);
  return fresh;
}
