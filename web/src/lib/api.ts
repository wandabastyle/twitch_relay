export interface SessionStateResponse {
  authenticated: boolean;
}

export interface ChannelsResponse {
  channels: Array<{ login: string }>;
}

export interface WatchTicketResponse {
  watch_url: string;
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

export async function getChannels(): Promise<Array<{ login: string }>> {
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
    (item): item is { login: string } => isObject(item) && typeof item.login === 'string'
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
  if (
    !isObject(payload) ||
    typeof payload.watch_url !== 'string'
  ) {
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
