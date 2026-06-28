import { isObject, readApiError, request, safeJson } from './core.js';
import type { WatchSessionResponse } from './types.js';

const parseWatchSessionPayload = (payload: unknown): WatchSessionResponse => {
  if (!isObject(payload)) {
    throw new Error('watch session payload is invalid');
  }

  const {
    app_version: appVersion,
    channel,
    delivery_mode: deliveryMode,
    display_name: displayName,
    game,
    live,
    manifest_url: manifestUrl,
    profile_url: profileUrl,
    relay,
    resolver,
    title,
    viewer_count: viewerCount,
  } = payload;

  if (
    typeof appVersion !== 'string' ||
    typeof channel !== 'string' ||
    typeof manifestUrl !== 'string' ||
    typeof relay !== 'boolean' ||
    typeof live !== 'boolean' ||
    typeof resolver !== 'string' ||
    typeof deliveryMode !== 'string'
  ) {
    throw new TypeError('watch session payload is invalid');
  }

  if (
    (resolver !== 'native' && resolver !== 'streamlink' && resolver !== 'auto') ||
    (deliveryMode !== 'cdn_first' && deliveryMode !== 'relay')
  ) {
    throw new TypeError('watch session payload is invalid');
  }

  return {
    app_version: appVersion,
    channel,
    delivery_mode: deliveryMode,
    display_name: typeof displayName === 'string' ? displayName : undefined,
    game: typeof game === 'string' ? game : undefined,
    live,
    manifest_url: manifestUrl,
    profile_url: typeof profileUrl === 'string' ? profileUrl : undefined,
    relay,
    resolver,
    title: typeof title === 'string' ? title : undefined,
    viewer_count: typeof viewerCount === 'number' ? viewerCount : undefined,
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
