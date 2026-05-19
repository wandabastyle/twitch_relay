import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  buildUrl,
  createRoutePattern,
  getRouteParams,
  isRoute,
  matchRoute,
  ROUTES,
} from './routes';

describe('Router Routes', () => {
  const DEFAULT_MOCK_HISTORY_LENGTH = 2;

  beforeEach(() => {
    // Mock window.location for SSR environment
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
  });

  describe('createRoutePattern', () => {
    it('should create pattern without params', () => {
      const route = createRoutePattern('/twitch');
      expect(route.path).toBe('/twitch');
      expect(route.paramNames).toEqual([]);
      expect(route.pattern.test('/twitch')).toBe(true);
      expect(route.pattern.test('/twitch/')).toBe(false);
    });

    it('should create pattern with single param', () => {
      const route = createRoutePattern('/twitch/channels/:login');
      expect(route.paramNames).toEqual(['login']);
      expect(route.pattern.test('/twitch/channels/testuser')).toBe(true);
      expect(route.pattern.test('/twitch/channels/')).toBe(false);
    });

    it('should create pattern with multiple params', () => {
      const route = createRoutePattern('/api/:version/:resource');
      expect(route.paramNames).toEqual(['version', 'resource']);
    });
  });

  describe('matchRoute', () => {
    it('should match root path', () => {
      const matchResult = matchRoute('/');
      expect(matchResult).toBeTruthy();
      expect(matchResult?.matched).toBe('/');
    });

    it('should match Twitch home', () => {
      const matchResult = matchRoute('/twitch');
      expect(matchResult).toBeTruthy();
      expect(matchResult?.matched).toBe('/twitch');
    });

    it('should match channel settings with params', () => {
      const matchResult = matchRoute('/twitch/channels/testuser');
      expect(matchResult).toBeTruthy();
      expect(matchResult?.params.login).toBe('testuser');
    });

    it('should match recordings overview', () => {
      const matchResult = matchRoute('/twitch/recordings');
      expect(matchResult).toBeTruthy();
      expect(matchResult?.matched).toBe('/twitch/recordings');
    });

    it('should match recordings play', () => {
      const matchResult = matchRoute('/twitch/recordings/play');
      expect(matchResult).toBeTruthy();
    });

    it('should match watch page with ticket', () => {
      const matchResult = matchRoute('/watch/abc123');
      expect(matchResult).toBeTruthy();
      expect(matchResult?.params.ticket).toBe('abc123');
    });

    it('should match QR login with token', () => {
      const matchResult = matchRoute('/qr-login/my-token-123');
      expect(matchResult).toBeTruthy();
      expect(matchResult?.params.token).toBe('my-token-123');
    });

    it('should match YouTube routes', () => {
      expect(matchRoute('/youtube')).toBeTruthy();
      expect(matchRoute('/youtube/recent')).toBeTruthy();
      expect(matchRoute('/youtube/playlists')).toBeTruthy();
    });

    it('should match YouTube channel with ID', () => {
      const matchResult = matchRoute('/youtube/channel/UCxxxx123');
      expect(matchResult).toBeTruthy();
      expect(matchResult?.params.channel_id).toBe('UCxxxx123');
    });

    it('should match YouTube playlist with ID', () => {
      const matchResult = matchRoute('/youtube/playlist/PLabc123');
      expect(matchResult).toBeTruthy();
      expect(matchResult?.params.playlist_id).toBe('PLabc123');
    });

    it('should match YouTube watch with video ID', () => {
      const matchResult = matchRoute('/youtube/watch/abc123xyz');
      expect(matchResult).toBeTruthy();
      expect(matchResult?.params.video_id).toBe('abc123xyz');
    });

    it('should decode URL-encoded params', () => {
      const matchResult = matchRoute('/twitch/channels/user%20name');
      expect(matchResult?.params.login).toBe('user name');
    });

    it('should return null for unknown routes', () => {
      expect(matchRoute('/unknown/path')).toBeNull();
      expect(matchRoute('/admin')).toBeNull();
    });
  });

  describe('isRoute', () => {
    it('should check if path matches pattern', () => {
      expect(isRoute('/twitch/channels/testuser', '/twitch/channels/:login')).toBe(true);
      expect(isRoute('/twitch/channels/testuser', '/twitch')).toBe(false);
      expect(isRoute('/youtube', '/youtube')).toBe(true);
    });
  });

  describe('getRouteParams', () => {
    it('should extract params from path', () => {
      const params = getRouteParams('/watch/ticket-123', '/watch/:ticket');
      expect(params?.ticket).toBe('ticket-123');
    });

    it('should return null for non-matching path', () => {
      const params = getRouteParams('/twitch', '/watch/:ticket');
      expect(params).toBeNull();
    });
  });

  describe('ROUTES constants', () => {
    it('should generate Twitch channel URL', () => {
      expect(ROUTES.TWITCH_CHANNEL('testuser')).toBe('/twitch/channels/testuser');
    });

    it('should encode URL components', () => {
      expect(ROUTES.TWITCH_CHANNEL('user name')).toBe('/twitch/channels/user%20name');
    });

    it('should generate watch URL', () => {
      expect(ROUTES.WATCH('abc-123')).toBe('/watch/abc-123');
    });

    it('should generate QR login URL', () => {
      expect(ROUTES.QR_LOGIN('token-xyz')).toBe('/qr-login/token-xyz');
    });

    it('should generate YouTube URLs', () => {
      expect(ROUTES.YOUTUBE_CHANNEL('UCxxx')).toBe('/youtube/channel/UCxxx');
      expect(ROUTES.YOUTUBE_PLAYLIST('PLxxx')).toBe('/youtube/playlist/PLxxx');
      expect(ROUTES.YOUTUBE_WATCH('abc123')).toBe('/youtube/watch/abc123');
    });

    it('should have static routes', () => {
      expect(ROUTES.HOME).toBe('/');
      expect(ROUTES.TWITCH).toBe('/twitch');
      expect(ROUTES.TWITCH_RECORDINGS).toBe('/twitch/recordings');
      expect(ROUTES.YOUTUBE).toBe('/youtube');
      expect(ROUTES.YOUTUBE_RECENT).toBe('/youtube/recent');
      expect(ROUTES.YOUTUBE_PLAYLISTS).toBe('/youtube/playlists');
    });
  });

  describe('buildUrl', () => {
    it('should build URLs with query params', () => {
      const url = buildUrl('/twitch/recordings/play', {
        channel_login: 'testuser',
        filename: 'recording.mp4',
      });
      expect(url).toContain('/twitch/recordings/play');
      expect(url).toContain('channel_login=testuser');
      expect(url).toContain('filename=recording.mp4');
    });

    it('should skip undefined params', () => {
      const url = buildUrl('/twitch', {
        optional: undefined,
        present: 'value',
      });
      expect(url).toContain('present=value');
      expect(url).not.toContain('optional');
    });
  });
});
