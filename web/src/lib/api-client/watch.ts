import { isObject, readApiError, request, safeJson } from './core.js';
import type { WatchSessionResponse } from './types.js';

const parseWatchSessionPayload = (payload: unknown): WatchSessionResponse => {
  if (
    !isObject(payload) ||
    typeof payload.app_version !== 'string' ||
    typeof payload.channel !== 'string' ||
    typeof payload.manifest_url !== 'string' ||
    typeof payload.relay !== 'boolean'
  ) {
    throw new Error('watch session payload is invalid');
  }

  return {
    app_version: payload.app_version,
    channel: payload.channel,
    manifest_url: payload.manifest_url,
    relay: payload.relay,
  };
};

export const getWatchSession = async (
  ticket: string,
  relay = false,
): Promise<WatchSessionResponse> => {
  const query = relay ? '?force=1' : '';
  const response = await request(`/api/watch-session/${encodeURIComponent(ticket)}${query}`);
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }
  return parseWatchSessionPayload(await safeJson(response));
};
