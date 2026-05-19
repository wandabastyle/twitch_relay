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
  matched: string;
  name: string;
  params: RouteParams;
}

export interface RouteDefinition {
  name: string;
  paramNames: string[];
  path: string;
  pattern: RegExp;
}

export type NavigationType = 'goto' | 'popstate' | 'redirect' | 'replace';

export interface HistoryState {
  [key: string]: unknown;
  youtubeReturnUrl?: string;
}

export interface NavigationOptions {
  replace?: boolean;
  state?: HistoryState;
}

export interface PageStore {
  params: RouteParams;
  path: string;
  query: QueryParams;
  state: HistoryState | null;
  url: URL;
}

// ============================================================================
// Constants
// ============================================================================

const FIRST_PARAM_INDEX = 1;

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
export const createRoutePattern = (path: string): RouteDefinition => {
  const paramNames: string[] = [];

  // Process the path: escape special chars, then replace :param with capture groups
  let regexPattern = path
    // Escape special regex chars
    .replace(/[.+*?^${}()|[\]\\]/g, '\\$&')
    // Replace :paramName with capture group
    .replace(/:([^/]+)/g, (match_, name) => {
      paramNames.push(name);
      return '([^/]+)';
    });

  // Ensure exact match
  regexPattern = `^${regexPattern}$`;

  return {
    // Will be set by createNamedRoutePattern
    name: '',
    paramNames,
    path,
    pattern: new RegExp(regexPattern),
  };
};

/**
 * Creates a named route definition.
 */
export const createNamedRoutePattern = (name: string, path: string): RouteDefinition => {
  const pattern = createRoutePattern(path);
  return {
    ...pattern,
    name,
  };
};

// ============================================================================
// Route Definitions with Names
// ============================================================================

interface NamedRouteDefinition {
  definition: RouteDefinition;
  name: string;
}

// Route definitions in order of specificity (most specific first)
const ROUTE_DEFINITIONS: NamedRouteDefinition[] = [
  // Root redirect
  { definition: createNamedRoutePattern('index', '/'), name: 'index' },

  // Twitch routes
  { definition: createNamedRoutePattern('twitch', '/twitch'), name: 'twitch' },
  {
    definition: createNamedRoutePattern('twitch_channel', '/twitch/channels/:login'),
    name: 'twitch_channel',
  },
  {
    definition: createNamedRoutePattern('twitch_recordings', '/twitch/recordings'),
    name: 'twitch_recordings',
  },
  {
    definition: createNamedRoutePattern('twitch_recordings_play', '/twitch/recordings/play'),
    name: 'twitch_recordings_play',
  },

  // Watch routes
  { definition: createNamedRoutePattern('watch', '/watch/:ticket'), name: 'watch' },

  // QR login
  { definition: createNamedRoutePattern('qr_login', '/qr-login/:token'), name: 'qr_login' },

  // YouTube routes
  { definition: createNamedRoutePattern('youtube', '/youtube'), name: 'youtube' },
  {
    definition: createNamedRoutePattern('youtube_recent', '/youtube/recent'),
    name: 'youtube_recent',
  },
  {
    definition: createNamedRoutePattern('youtube_playlists', '/youtube/playlists'),
    name: 'youtube_playlists',
  },
  {
    definition: createNamedRoutePattern('youtube_channel', '/youtube/channel/:channel_id'),
    name: 'youtube_channel',
  },
  {
    definition: createNamedRoutePattern('youtube_playlist', '/youtube/playlist/:playlist_id'),
    name: 'youtube_playlist',
  },
  {
    definition: createNamedRoutePattern('youtube_watch', '/youtube/watch/:video_id'),
    name: 'youtube_watch',
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
export const matchRoute = (path: string): RouteMatch | null => {
  for (const route of ROUTE_DEFINITIONS) {
    const match = path.match(route.definition.pattern);
    if (match) {
      const params: RouteParams = {};
      for (let index = 0; index < route.definition.paramNames.length; index += 1) {
        const value = match[index + FIRST_PARAM_INDEX];
        if (value !== undefined) {
          params[route.definition.paramNames[index]] = decodeURIComponent(value);
        }
      }
      return {
        matched: route.definition.path,
        name: route.name,
        params,
      };
    }
  }
  return null;
};

/**
 * Checks if a path matches the given route pattern.
 * @param path - The URL path to check
 * @param pattern - Route pattern like /twitch/channels/:login
 * @returns True if the path matches the pattern
 */
export const isRoute = (path: string, pattern: string): boolean => {
  const route = createRoutePattern(pattern);
  return route.pattern.test(path);
};

/**
 * Gets route parameters for a specific path.
 * @param path - The URL path
 * @param pattern - Route pattern like /twitch/channels/:login
 * @returns Params object or null if no match
 */
export const getRouteParams = (path: string, pattern: string): RouteParams | null => {
  const route = createRoutePattern(pattern);
  const match = path.match(route.pattern);
  if (!match) {
    return null;
  }

  const params: RouteParams = {};
  for (let index = 0; index < route.paramNames.length; index += 1) {
    const value = match[index + FIRST_PARAM_INDEX];
    if (value !== undefined) {
      params[route.paramNames[index]] = decodeURIComponent(value);
    }
  }
  return params;
};

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
export const buildUrl = (
  basePath: string,
  params: Record<string, number | string | undefined>,
  origin?: string,
): string => {
  const url = new URL(basePath, origin ?? 'http://localhost');
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined) {
      url.searchParams.set(key, String(value));
    }
  }
  return url.pathname + url.search;
};

// ============================================================================
// Route Constants
// ============================================================================

export const ROUTES = {
  // Root
  HOME: '/',
  HOME_REDIRECT: '/twitch',

  // QR Login
  QR_LOGIN: (token: string) => `/qr-login/${encodeURIComponent(token)}`,

  // Twitch
  TWITCH: '/twitch',
  TWITCH_CHANNEL: (login: string) => `/twitch/channels/${encodeURIComponent(login)}`,
  TWITCH_RECORDINGS: '/twitch/recordings',
  TWITCH_RECORDINGS_PLAY: '/twitch/recordings/play',

  // Watch
  WATCH: (ticket: string) => `/watch/${encodeURIComponent(ticket)}`,

  // YouTube
  YOUTUBE: '/youtube',
  YOUTUBE_CHANNEL: (channelId: string) => `/youtube/channel/${encodeURIComponent(channelId)}`,
  YOUTUBE_PLAYLIST: (playlistId: string) => `/youtube/playlist/${encodeURIComponent(playlistId)}`,
  YOUTUBE_PLAYLISTS: '/youtube/playlists',
  YOUTUBE_RECENT: '/youtube/recent',
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
export const parseQueryParams = (searchParams: URLSearchParams): QueryParams => {
  const query: QueryParams = {};
  // Convert forEach to for-of loop
  for (const [key, value] of searchParams.entries()) {
    query[key] = value;
  }
  return query;
};
