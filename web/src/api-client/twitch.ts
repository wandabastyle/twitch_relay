import { isObject, readApiError, request, safeJson } from './core.js';
import type { TwitchStatusResponse } from './types.js';

const parseOptionalString = (value: unknown): string | undefined =>
  typeof value === 'string' ? value : undefined;

const parseScopes = (value: unknown): string[] =>
  Array.isArray(value)
    ? value.filter((scope: unknown): scope is string => typeof scope === 'string')
    : [];

const parseTwitchStatusPayload = (payload: unknown): TwitchStatusResponse => {
  if (!isObject(payload) || typeof payload.connected !== 'boolean') {
    throw new Error('twitch status payload is invalid');
  }
  return {
    connected: payload.connected,
    display_name: parseOptionalString(payload.display_name),
    login: parseOptionalString(payload.login),
    scopes: parseScopes(payload.scopes),
  };
};

export const getTwitchStatus = async (): Promise<TwitchStatusResponse> => {
  const response = await request('/api/twitch/status');
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }
  return parseTwitchStatusPayload(await safeJson(response));
};

export const getTwitchConnectUrl = (): string => '/api/twitch/connect';

export const disconnectTwitch = async (): Promise<void> => {
  const response = await request('/api/twitch/disconnect', { method: 'POST' });
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }
};
