import { isObject, readApiError, request, safeJson } from './core.js';
import type { TwitchStatusResponse } from './types.js';

export const getTwitchStatus = async (): Promise<TwitchStatusResponse> => {
  const response = await request('/api/twitch/status');
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.connected !== 'boolean') {
    throw new Error('twitch status payload is invalid');
  }

  let displayName: string | undefined = undefined;
  if (typeof payload.display_name === 'string') {
    ({ display_name: displayName } = payload);
  }

  let login: string | undefined = undefined;
  if (typeof payload.login === 'string') {
    ({ login } = payload);
  }

  let scopes: string[] = [];
  if (Array.isArray(payload.scopes)) {
    scopes = payload.scopes.filter((scope): scope is string => typeof scope === 'string');
  }

  return {
    connected: payload.connected,
    display_name: displayName,
    login,
    scopes,
  };
};

export const getTwitchConnectUrl = (): string => '/api/twitch/connect';

export const disconnectTwitch = async (): Promise<void> => {
  const response = await request('/api/twitch/disconnect', { method: 'POST' });
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }
};
