import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  buildUrl,
  createRoutePattern,
  getRouteParams,
  isRoute,
  matchRoute,
  normalizeHistoryState,
  parseHistoryState,
  ROUTES,
} from './routes';

const DEFAULT_MOCK_HISTORY_LENGTH = 2;
const INVALID_NUMERIC_STATE = 42;

const installWindowMock = (): void => {
  vi.stubGlobal('window', {
    addEventListener: vi.fn(),
    history: {
      back: vi.fn(),
      length: DEFAULT_MOCK_HISTORY_LENGTH,
      pushState: vi.fn(),
      replaceState: vi.fn(),
      state: null,
    },
    location: {
      href: 'http://localhost/twitch',
      origin: 'http://localhost',
      pathname: '/twitch',
      search: '',
    },
  });
};

const matchRouteExpectations: readonly (readonly [string, string, string?])[] = [
  ['/', '/'],
  ['/twitch', '/twitch'],
  ['/twitch/channels/testuser', '/twitch/channels/:login', 'testuser'],
  ['/twitch/recordings', '/twitch/recordings'],
  ['/twitch/recordings/play', '/twitch/recordings/play'],
  ['/watch/abc123', '/watch/:ticket', 'abc123'],
  ['/qr-login/my-token-123', '/qr-login/:token', 'my-token-123'],
  ['/youtube/channel/UCxxxx123', '/youtube/channel/:channel_id', 'UCxxxx123'],
  ['/youtube/playlist/PLabc123', '/youtube/playlist/:playlist_id', 'PLabc123'],
  ['/youtube/watch/abc123xyz', '/youtube/watch/:video_id', 'abc123xyz'],
];

describe('createRoutePattern', () => {
  beforeEach(installWindowMock);

  it('creates pattern without params', () => {
    const route = createRoutePattern('/twitch');
    expect(route.path).toBe('/twitch');
    expect(route.paramNames).toEqual([]);
    expect(route.pattern.test('/twitch')).toBe(true);
    expect(route.pattern.test('/twitch/')).toBe(false);
  });

  it('creates pattern with params', () => {
    const route = createRoutePattern('/api/:version/:resource');
    expect(route.paramNames).toEqual(['version', 'resource']);
    expect(route.pattern.test('/api/v1/users')).toBe(true);
  });
});

describe('matchRoute', () => {
  beforeEach(installWindowMock);

  it('matches known routes', () => {
    for (const [path, expectedMatch, expectedParam] of matchRouteExpectations) {
      const matchResult = matchRoute(path);
      expect(matchResult).toBeTruthy();
      expect(matchResult?.matched).toBe(expectedMatch);
      if (expectedParam !== undefined) {
        expect(Object.values(matchResult?.params ?? {})).toContain(expectedParam);
      }
    }
  });

  it('decodes url params and rejects unknown routes', () => {
    expect(matchRoute('/twitch/channels/user%20name')?.params.login).toBe('user name');
    expect(matchRoute('/unknown/path')).toBeNull();
    expect(matchRoute('/admin')).toBeNull();
  });

  it('matches static youtube routes', () => {
    expect(matchRoute('/youtube')).toBeTruthy();
    expect(matchRoute('/youtube/recent')).toBeTruthy();
    expect(matchRoute('/youtube/playlists')).toBeTruthy();
  });
});

describe('route helpers', () => {
  beforeEach(installWindowMock);

  it('checks and extracts params', () => {
    expect(isRoute('/twitch/channels/testuser', '/twitch/channels/:login')).toBe(true);
    expect(isRoute('/twitch/channels/testuser', '/twitch')).toBe(false);
    expect(isRoute('/youtube', '/youtube')).toBe(true);
    expect(getRouteParams('/watch/ticket-123', '/watch/:ticket')?.ticket).toBe('ticket-123');
    expect(getRouteParams('/twitch', '/watch/:ticket')).toBeNull();
  });
});

describe('ROUTES constants', () => {
  beforeEach(installWindowMock);

  it('builds dynamic urls', () => {
    expect(ROUTES.twitchChannel('testuser')).toBe('/twitch/channels/testuser');
    expect(ROUTES.twitchChannel('user name')).toBe('/twitch/channels/user%20name');
    expect(ROUTES.watch('abc-123')).toBe('/watch/abc-123');
    expect(ROUTES.qrLogin('token-xyz')).toBe('/qr-login/token-xyz');
    expect(ROUTES.youtubeChannel('UCxxx')).toBe('/youtube/channel/UCxxx');
    expect(ROUTES.youtubePlaylist('PLxxx')).toBe('/youtube/playlist/PLxxx');
    expect(ROUTES.youtubeWatch('abc123')).toBe('/youtube/watch/abc123');
  });

  it('defines static urls', () => {
    expect(ROUTES.HOME).toBe('/');
    expect(ROUTES.TWITCH).toBe('/twitch');
    expect(ROUTES.TWITCH_RECORDINGS).toBe('/twitch/recordings');
    expect(ROUTES.YOUTUBE).toBe('/youtube');
    expect(ROUTES.YOUTUBE_RECENT).toBe('/youtube/recent');
    expect(ROUTES.YOUTUBE_PLAYLISTS).toBe('/youtube/playlists');
  });
});

describe('buildUrl', () => {
  beforeEach(installWindowMock);

  it('builds query strings and skips undefined', () => {
    const fullUrl = buildUrl('/twitch/recordings/play', {
      channel_login: 'testuser',
      filename: 'recording.mp4',
    });
    expect(fullUrl).toContain('/twitch/recordings/play');
    expect(fullUrl).toContain('channel_login=testuser');
    expect(fullUrl).toContain('filename=recording.mp4');

    const partialUrl = buildUrl('/twitch', { optional: undefined, present: 'value' });
    expect(partialUrl).toContain('present=value');
    expect(partialUrl).not.toContain('optional');
  });
});

describe('history state helpers', () => {
  beforeEach(installWindowMock);

  it('parses valid and invalid history states', () => {
    const maybeUndefined: { value?: unknown } = {};
    expect(parseHistoryState(null)).toBeNull();
    expect(parseHistoryState(maybeUndefined.value)).toBeNull();
    expect(parseHistoryState('x')).toBeNull();
    expect(parseHistoryState(INVALID_NUMERIC_STATE)).toBeNull();
    expect(parseHistoryState(true)).toBeNull();
    expect(parseHistoryState({ youtubeReturnUrl: '/youtube/recent' })).toEqual({
      youtubeReturnUrl: '/youtube/recent',
    });
    expect(parseHistoryState({ youtubeReturnUrl: 123 })).toBeNull();
    expect(parseHistoryState({ youtubeReturnUrl: null })).toBeNull();
  });

  it('ignores inherited state and normalizes undefined', () => {
    const inherited: object = {};
    Object.setPrototypeOf(inherited, { youtubeReturnUrl: '/youtube' });
    const maybeUndefined: { value?: { youtubeReturnUrl?: string } } = {};

    expect(parseHistoryState(inherited)).toBeNull();
    expect(normalizeHistoryState(maybeUndefined.value)).toBeNull();
    expect(normalizeHistoryState({ youtubeReturnUrl: '/youtube' })).toEqual({
      youtubeReturnUrl: '/youtube',
    });
  });
});
