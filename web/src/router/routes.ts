export type RouteParams = Record<string, string>;

export type QueryParams = Record<string, string | null>;

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
  readonly [key: string]: unknown;
  readonly youtubeReturnUrl?: string;
}

export interface NavigationOptions {
  readonly replace?: boolean;
  readonly state?: HistoryState;
}

export interface PageStore {
  params: RouteParams;
  path: string;
  query: QueryParams;
  state: HistoryState | null;
  url: URL;
}

const FIRST_PARAM_INDEX = 1;
const INDEX_INCREMENT = 1;
export const createRoutePattern = (path: string): RouteDefinition => {
  const paramNames: string[] = [];

  let regexPattern = path
    .replaceAll(/[.+*?^${}()|[\]\\]/g, String.raw`\$\u0026`)
    .replaceAll(/:([^/]+)/g, (_match: string, name: string) => {
      paramNames.push(name);
      return '([^/]+)';
    });

  regexPattern = `^${regexPattern}$`;

  return {
    name: '',
    paramNames,
    path,
    pattern: new RegExp(regexPattern),
  };
};

export const createNamedRoutePattern = (name: string, path: string): RouteDefinition => {
  const pattern = createRoutePattern(path);
  return {
    name,
    paramNames: pattern.paramNames,
    path: pattern.path,
    pattern: pattern.pattern,
  };
};

interface NamedRouteDefinition {
  definition: RouteDefinition;
  name: string;
}

const ROUTE_DEFINITIONS: NamedRouteDefinition[] = [
  { definition: createNamedRoutePattern('index', '/'), name: 'index' },
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
  { definition: createNamedRoutePattern('watch', '/watch/:ticket'), name: 'watch' },
  { definition: createNamedRoutePattern('qr_login', '/qr-login/:token'), name: 'qr_login' },
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

export const matchRoute = (path: string): RouteMatch | null => {
  for (const route of ROUTE_DEFINITIONS) {
    const match = path.match(route.definition.pattern);
    if (match) {
      const params: RouteParams = {};
      for (let index = 0; index < route.definition.paramNames.length; index += INDEX_INCREMENT) {
        const value = match[index + FIRST_PARAM_INDEX];
        params[route.definition.paramNames[index]] = decodeURIComponent(value);
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

export const isRoute = (path: string, pattern: string): boolean => {
  const route = createRoutePattern(pattern);
  return route.pattern.test(path);
};

export const getRouteParams = (path: string, pattern: string): RouteParams | null => {
  const route = createRoutePattern(pattern);
  const match = path.match(route.pattern);
  if (!match) {
    return null;
  }

  const params: RouteParams = {};
  for (let index = 0; index < route.paramNames.length; index += INDEX_INCREMENT) {
    const value = match[index + FIRST_PARAM_INDEX];
    params[route.paramNames[index]] = decodeURIComponent(value);
  }
  return params;
};

export const buildUrl = (
  basePath: string,
  params: Readonly<Record<string, number | string | undefined>>,
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

export const ROUTES = {
  HOME: '/',
  HOME_REDIRECT: '/twitch',
  TWITCH: '/twitch',
  TWITCH_RECORDINGS: '/twitch/recordings',
  TWITCH_RECORDINGS_PLAY: '/twitch/recordings/play',
  YOUTUBE: '/youtube',
  YOUTUBE_PLAYLISTS: '/youtube/playlists',
  YOUTUBE_RECENT: '/youtube/recent',
  qrLogin: (token: string) => `/qr-login/${encodeURIComponent(token)}`,
  twitchChannel: (login: string) => `/twitch/channels/${encodeURIComponent(login)}`,
  watch: (ticket: string) => `/watch/${encodeURIComponent(ticket)}`,
  youtubeChannel: (channelId: string) => `/youtube/channel/${encodeURIComponent(channelId)}`,
  youtubePlaylist: (playlistId: string) => `/youtube/playlist/${encodeURIComponent(playlistId)}`,
  youtubeWatch: (videoId: string) => `/youtube/watch/${encodeURIComponent(videoId)}`,
} as const;

export const parseQueryParams = (searchParams: Readonly<URLSearchParams>): QueryParams => {
  const query: QueryParams = {};
  for (const [key, value] of searchParams.entries()) {
    query[key] = value;
  }
  return query;
};

export const parseHistoryState = (rawState: unknown): HistoryState | null => {
  if (typeof rawState !== 'object' || rawState === null) {
    return null;
  }

  const descriptor = Object.getOwnPropertyDescriptor(rawState, 'youtubeReturnUrl');
  if (descriptor !== undefined && typeof descriptor.value === 'string') {
    return { youtubeReturnUrl: descriptor.value };
  }

  return null;
};

export const normalizeHistoryState = (
  state: NavigationOptions['state'] | undefined,
): HistoryState | null => state ?? null;

export const navigate = (targetPath: string, options: NavigationOptions = {}): void => {
  const { replace = false, state } = options;

  if (replace) {
    window.history.replaceState(normalizeHistoryState(state), '', targetPath);
  } else {
    window.history.pushState(normalizeHistoryState(state), '', targetPath);
  }

  // Dispatch popstate event to trigger route updates
  window.dispatchEvent(new PopStateEvent('popstate'));
};

export const goBack = (): void => {
  if (window.history.length > 1) {
    window.history.back();
  } else {
    navigate('/twitch');
  }
};
