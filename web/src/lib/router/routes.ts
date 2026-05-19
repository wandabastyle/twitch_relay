/**
 * Client-side router utilities (pure functions, no Svelte dependencies)
 * These functions can be tested in isolation without Svelte compilation.
 */

// ============================================================================
// Types
// ============================================================================

export interface RouteParams {
  [key: string]: string;
}

export interface QueryParams {
  [key: string]: string | null;
}

export interface RouteMatch {
  params: RouteParams;
  matched: string;
  name: string;
}

export interface RouteDefinition {
  pattern: RegExp;
  paramNames: string[];
  path: string;
  name: string;
}

export type NavigationType = 'goto' | 'popstate' | 'replace' | 'redirect';

export interface HistoryState {
  youtubeReturnUrl?: string;
  [key: string]: unknown;
}

export interface NavigationOptions {
  replace?: boolean;
  state?: HistoryState;
}

export interface PageStore {
  path: string;
  params: RouteParams;
  query: QueryParams;
  state: HistoryState | null;
  url: URL;
}

// ============================================================================
// Route Pattern Creation
// ============================================================================

/**
 * Converts a route pattern to a regex and extracts param names.
 * Patterns:
 *   - /twitch/channels/:login  -> matches /twitch/channels/someuser
 *   - /watch/:ticket           -> matches /watch/abc123
 *   - /youtube/playlist/:playlist_id -> matches /youtube/playlist/PLabc
 *
 * @param path - Route pattern like /twitch/channels/:login
 * @returns Object with regex pattern and param names
 */
export function createRoutePattern(path: string): RouteDefinition {
  const paramNames: string[] = [];

  // Process the path: escape special chars, then replace :param with capture groups
  let regexPattern = path
    .replace(/[.+*?^${}()|[\]\\]/g, '\\$&') // Escape special regex chars
    .replace(/:([^/]+)/g, (_, name) => {
      // Replace :paramName with capture group
      paramNames.push(name);
      return '([^/]+)';
    });

  // Ensure exact match
  regexPattern = '^' + regexPattern + '$';

  return {
    pattern: new RegExp(regexPattern),
    paramNames,
    path,
    name: '', // Will be set by createNamedRoutePattern
  };
}

/**
 * Creates a named route definition.
 */
export function createNamedRoutePattern(name: string, path: string): RouteDefinition {
  const pattern = createRoutePattern(path);
  return {
    ...pattern,
    name,
  };
}

// ============================================================================
// Route Definitions with Names
// ============================================================================

interface NamedRouteDefinition {
  name: string;
  definition: RouteDefinition;
}

// Route definitions in order of specificity (most specific first)
const ROUTE_DEFINITIONS: NamedRouteDefinition[] = [
  // Root redirect
  { name: 'index', definition: createNamedRoutePattern('index', '/') },

  // Twitch routes
  { name: 'twitch', definition: createNamedRoutePattern('twitch', '/twitch') },
  {
    name: 'twitch_channel',
    definition: createNamedRoutePattern('twitch_channel', '/twitch/channels/:login'),
  },
  {
    name: 'twitch_recordings',
    definition: createNamedRoutePattern('twitch_recordings', '/twitch/recordings'),
  },
  {
    name: 'twitch_recordings_play',
    definition: createNamedRoutePattern('twitch_recordings_play', '/twitch/recordings/play'),
  },

  // Watch routes
  { name: 'watch', definition: createNamedRoutePattern('watch', '/watch/:ticket') },

  // QR login
  { name: 'qr_login', definition: createNamedRoutePattern('qr_login', '/qr-login/:token') },

  // YouTube routes
  { name: 'youtube', definition: createNamedRoutePattern('youtube', '/youtube') },
  {
    name: 'youtube_recent',
    definition: createNamedRoutePattern('youtube_recent', '/youtube/recent'),
  },
  {
    name: 'youtube_playlists',
    definition: createNamedRoutePattern('youtube_playlists', '/youtube/playlists'),
  },
  {
    name: 'youtube_channel',
    definition: createNamedRoutePattern('youtube_channel', '/youtube/channel/:channel_id'),
  },
  {
    name: 'youtube_playlist',
    definition: createNamedRoutePattern('youtube_playlist', '/youtube/playlist/:playlist_id'),
  },
  {
    name: 'youtube_watch',
    definition: createNamedRoutePattern('youtube_watch', '/youtube/watch/:video_id'),
  },
];

// ============================================================================
// Route Matching Functions
// ============================================================================

/**
 * Matches a path against the route definitions.
 * @param path - The URL path to match
 * @returns The matched route with params and name, or null if no match
 */
export function matchRoute(path: string): RouteMatch | null {
  for (const route of ROUTE_DEFINITIONS) {
    const match = path.match(route.definition.pattern);
    if (match) {
      const params: RouteParams = {};
      for (let i = 0; i < route.definition.paramNames.length; i++) {
        const value = match[i + 1];
        if (value !== undefined) {
          params[route.definition.paramNames[i]] = decodeURIComponent(value);
        }
      }
      return {
        params,
        matched: route.definition.path,
        name: route.name,
      };
    }
  }
  return null;
}

/**
 * Checks if a path matches the given route pattern.
 * @param path - The URL path to check
 * @param pattern - Route pattern like /twitch/channels/:login
 * @returns True if the path matches the pattern
 */
export function isRoute(path: string, pattern: string): boolean {
  const route = createRoutePattern(pattern);
  return route.pattern.test(path);
}

/**
 * Gets route parameters for a specific path.
 * @param path - The URL path
 * @param pattern - Route pattern like /twitch/channels/:login
 * @returns Params object or null if no match
 */
export function getRouteParams(path: string, pattern: string): RouteParams | null {
  const route = createRoutePattern(pattern);
  const match = path.match(route.pattern);
  if (!match) return null;

  const params: RouteParams = {};
  for (let i = 0; i < route.paramNames.length; i++) {
    const value = match[i + 1];
    if (value !== undefined) {
      params[route.paramNames[i]] = decodeURIComponent(value);
    }
  }
  return params;
}

// ============================================================================
// URL Builder
// ============================================================================

/**
 * Builds a URL with query parameters.
 *
 * @param basePath - The base path
 * @param params - Query parameters to add
 * @param origin - Optional origin (defaults to current window location or localhost)
 * @returns The full URL string
 *
 * @example
 * ```ts
 * const url = buildUrl('/twitch/recordings/play', {
 *   channel_login: 'twitchuser',
 *   filename: 'recording.mp4'
 * });
 * // Returns: /twitch/recordings/play?channel_login=twitchuser&filename=recording.mp4
 * ```
 */
export function buildUrl(
  basePath: string,
  params: Record<string, string | number | undefined>,
  origin?: string,
): string {
  const url = new URL(basePath, origin ?? 'http://localhost');
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined) {
      url.searchParams.set(key, String(value));
    }
  }
  return url.pathname + url.search;
}

// ============================================================================
// Route Constants
// ============================================================================

export const ROUTES = {
  // Root
  HOME: '/',
  HOME_REDIRECT: '/twitch',

  // Twitch
  TWITCH: '/twitch',
  TWITCH_CHANNEL: (login: string) => `/twitch/channels/${encodeURIComponent(login)}`,
  TWITCH_RECORDINGS: '/twitch/recordings',
  TWITCH_RECORDINGS_PLAY: '/twitch/recordings/play',

  // Watch
  WATCH: (ticket: string) => `/watch/${encodeURIComponent(ticket)}`,

  // QR Login
  QR_LOGIN: (token: string) => `/qr-login/${encodeURIComponent(token)}`,

  // YouTube
  YOUTUBE: '/youtube',
  YOUTUBE_RECENT: '/youtube/recent',
  YOUTUBE_PLAYLISTS: '/youtube/playlists',
  YOUTUBE_CHANNEL: (channelId: string) => `/youtube/channel/${encodeURIComponent(channelId)}`,
  YOUTUBE_PLAYLIST: (playlistId: string) => `/youtube/playlist/${encodeURIComponent(playlistId)}`,
  YOUTUBE_WATCH: (videoId: string) => `/youtube/watch/${encodeURIComponent(videoId)}`,
} as const;

// ============================================================================
// Query Parameter Parsing
// ============================================================================

/**
 * Parses URLSearchParams into a plain object.
 * @param searchParams - URLSearchParams to parse
 * @returns Plain object with query parameters
 */
export function parseQueryParams(searchParams: URLSearchParams): QueryParams {
  const query: QueryParams = {};
  searchParams.forEach((value, key) => {
    query[key] = value;
  });
  return query;
}
