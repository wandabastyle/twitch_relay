export interface SessionStateResponse {
  authenticated: boolean;
}

export interface ChannelEntry {
  login: string;
  image_url?: string;
  display_name?: string;
  source: 'manual' | 'followed' | 'both';
  removable: boolean;
}

export interface ChannelsResponse {
  channels: Array<ChannelEntry>;
}

export interface WatchTicketResponse {
  watch_url: string;
}

export interface TwitchStatusResponse {
  connected: boolean;
  login?: string;
  display_name?: string;
  scopes: string[];
}

interface ErrorPayload {
  error?: string;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function readError(payload: unknown): string {
  if (isObject(payload) && typeof payload.error === 'string') {
    return payload.error;
  }
  return 'request failed';
}

async function safeJson(response: Response): Promise<unknown> {
  try {
    return (await response.json()) as unknown;
  } catch {
    return null;
  }
}

async function request(input: string, init?: RequestInit): Promise<Response> {
  return fetch(input, {
    credentials: 'same-origin',
    ...init
  });
}

export async function getSessionState(): Promise<boolean> {
  const response = await request('/auth/session');
  if (!response.ok) {
    throw new Error(`session request failed (${String(response.status)})`);
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.authenticated !== 'boolean') {
    throw new Error('session response payload is invalid');
  }

  return payload.authenticated;
}

export async function login(accessCode: string): Promise<void> {
  const response = await request('/auth/login', {
    method: 'POST',
    headers: {
      'content-type': 'application/json'
    },
    body: JSON.stringify({ access_code: accessCode })
  });

  if (response.ok) {
    return;
  }

  const payload = (await safeJson(response)) as ErrorPayload;
  throw new Error(payload.error ?? 'login failed');
}

export async function logout(): Promise<void> {
  await request('/auth/logout', { method: 'POST' });
}

export async function getChannels(): Promise<Array<ChannelEntry>> {
  const response = await request('/api/channels');
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
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
      typeof item.removable === 'boolean'
  );

  return channels;
}

export async function createWatchTicket(channelLogin: string): Promise<WatchTicketResponse> {
  const response = await request('/api/watch-ticket', {
    method: 'POST',
    headers: {
      'content-type': 'application/json'
    },
    body: JSON.stringify({ channel_login: channelLogin })
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.watch_url !== 'string') {
    throw new Error('watch ticket payload is invalid');
  }

  return {
    watch_url: payload.watch_url
  };
}

export async function addChannel(login: string): Promise<void> {
  const response = await request('/api/channels', {
    method: 'POST',
    headers: {
      'content-type': 'application/json'
    },
    body: JSON.stringify({ login })
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }
}

export async function removeChannel(login: string): Promise<void> {
  const response = await request(`/api/channels/${encodeURIComponent(login)}`, {
    method: 'DELETE'
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }
}

export interface ChannelStatus {
  live: boolean;
  viewer_count?: number;
  game?: string;
  title?: string;
  profile_url?: string;
  display_name?: string;
}

export interface LiveStatusResponse {
  channels: Record<string, ChannelStatus>;
}

export async function getLiveStatus(): Promise<LiveStatusResponse> {
  const response = await request('/api/live-status');
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !isObject(payload.channels)) {
    throw new Error('live status payload is invalid');
  }

  return {
    channels: payload.channels as Record<string, ChannelStatus>
  };
}

export async function getTwitchStatus(): Promise<TwitchStatusResponse> {
  const response = await request('/api/twitch/status');
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.connected !== 'boolean') {
    throw new Error('twitch status payload is invalid');
  }

  return {
    connected: payload.connected,
    login: typeof payload.login === 'string' ? payload.login : undefined,
    display_name: typeof payload.display_name === 'string' ? payload.display_name : undefined,
    scopes: Array.isArray(payload.scopes) ? payload.scopes.filter((scope): scope is string => typeof scope === 'string') : []
  };
}

export function getTwitchConnectUrl(): string {
  return '/api/twitch/connect';
}

export async function disconnectTwitch(): Promise<void> {
  const response = await request('/api/twitch/disconnect', { method: 'POST' });
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }
}
