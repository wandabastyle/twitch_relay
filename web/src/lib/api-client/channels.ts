import { isObject, safeJson, readError, request } from "./core";
import type { ChannelEntry, LiveStatusResponse, ChannelStatus, WatchTicketResponse } from "./types";

const LIVE_STATUS_CACHE_KEY = "twitchRelay.liveStatus";
const LIVE_STATUS_CACHE_MAX_AGE_MS = 60000;

const CHANNELS_CACHE_KEY = "twitchRelay.channels";
const CHANNELS_CACHE_MAX_AGE_MS = 5 * 60 * 1000; // 5 minutes

interface LiveStatusCacheEntry {
  timestamp: number;
  data: LiveStatusResponse;
}

interface ChannelsCacheEntry {
  timestamp: number;
  data: Array<ChannelEntry>;
}

function getChannelsFromCache(): Array<ChannelEntry> | null {
  if (typeof window === "undefined") {
    return null;
  }

  try {
    const encoded = window.sessionStorage.getItem(CHANNELS_CACHE_KEY);
    if (!encoded) {
      return null;
    }

    const parsed = JSON.parse(encoded) as unknown;
    if (!isObject(parsed) || typeof parsed.timestamp !== "number" || !Array.isArray(parsed.data)) {
      return null;
    }

    const ageMs = Date.now() - parsed.timestamp;
    if (ageMs > CHANNELS_CACHE_MAX_AGE_MS) {
      return null;
    }

    // Validate cache entries
    const channels = parsed.data.filter(
      (item): item is ChannelEntry =>
        isObject(item) &&
        typeof item.login === "string" &&
        (item.source === "manual" || item.source === "followed" || item.source === "both") &&
        typeof item.removable === "boolean",
    );

    return channels;
  } catch {
    return null;
  }
}

function setChannelsCache(data: Array<ChannelEntry>): void {
  if (typeof window === "undefined") {
    return;
  }

  try {
    const payload: ChannelsCacheEntry = {
      timestamp: Date.now(),
      data,
    };
    window.sessionStorage.setItem(CHANNELS_CACHE_KEY, JSON.stringify(payload));
  } catch {
    // Ignore storage failures
  }
}

export function clearChannelsCache(): void {
  if (typeof window === "undefined") {
    return;
  }

  try {
    window.sessionStorage.removeItem(CHANNELS_CACHE_KEY);
  } catch {
    // Ignore storage failures
  }
}

export async function getChannels(): Promise<Array<ChannelEntry>> {
  const response = await request("/api/channels");
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
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
    throw new Error(readError(payload));
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
    throw new Error(readError(payload));
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
    throw new Error(readError(payload));
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
  if (typeof window === "undefined") {
    return null;
  }

  try {
    const encoded = window.sessionStorage.getItem(LIVE_STATUS_CACHE_KEY);
    if (!encoded) {
      return null;
    }

    const parsed = JSON.parse(encoded) as unknown;
    if (!isObject(parsed) || typeof parsed.timestamp !== "number" || !("data" in parsed)) {
      return null;
    }

    const ageMs = Date.now() - parsed.timestamp;
    if (ageMs > LIVE_STATUS_CACHE_MAX_AGE_MS) {
      return null;
    }

    return parseLiveStatusPayload(parsed.data);
  } catch {
    return null;
  }
}

function setLiveStatusCache(data: LiveStatusResponse): void {
  if (typeof window === "undefined") {
    return;
  }

  try {
    const payload: LiveStatusCacheEntry = {
      timestamp: Date.now(),
      data,
    };
    window.sessionStorage.setItem(LIVE_STATUS_CACHE_KEY, JSON.stringify(payload));
  } catch {
    // Ignore storage failures and continue with in-memory state.
  }
}

async function fetchLiveStatusFromApi(): Promise<LiveStatusResponse> {
  const response = await request("/api/live-status");
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
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
