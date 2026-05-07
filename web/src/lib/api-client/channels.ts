import { isObject, safeJson, readError, request } from "./core";
import type { ChannelEntry, LiveStatusResponse, ChannelStatus } from "./types";

const LIVE_STATUS_CACHE_KEY = "twitchRelay.liveStatus";
const LIVE_STATUS_CACHE_MAX_AGE_MS = 60000;

interface LiveStatusCacheEntry {
  timestamp: number;
  data: LiveStatusResponse;
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

  return channels;
}

export async function createWatchTicket(channelLogin: string): Promise<{ watch_url: string }> {
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
}

export async function removeChannel(login: string): Promise<void> {
  const response = await request(`/api/channels/${encodeURIComponent(login)}`, {
    method: "DELETE",
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }
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
