import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  matchRoute,
  isRoute,
  getRouteParams,
  buildUrl,
  ROUTES,
  createRoutePattern,
} from './routes';

describe('Router Routes', () => {
  beforeEach(() => {
    // Mock window.location for SSR environment
    vi.stubGlobal('window', {
      location: {
        pathname: '/twitch',
        href: 'http://localhost/twitch',
        origin: 'http://localhost',
        search: '',
      },
      history: {
        pushState: vi.fn(),
        replaceState: vi.fn(),
        back: vi.fn(),
        length: 2,
        state: null,
      },
      addEventListener: vi.fn(),
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
      const match = matchRoute('/');
      expect(match).toBeTruthy();
      expect(match?.matched).toBe('/');
    });

    it('should match Twitch home', () => {
      const match = matchRoute('/twitch');
      expect(match).toBeTruthy();
      expect(match?.matched).toBe('/twitch');
    });

    it('should match channel settings with params', () => {
      const match = matchRoute('/twitch/channels/testuser');
      expect(match).toBeTruthy();
      expect(match?.params.login).toBe('testuser');
    });

    it('should match recordings overview', () => {
      const match = matchRoute('/twitch/recordings');
      expect(match).toBeTruthy();
      expect(match?.matched).toBe('/twitch/recordings');
    });

    it('should match recordings play', () => {
      const match = matchRoute('/twitch/recordings/play');
      expect(match).toBeTruthy();
    });

    it('should match watch page with ticket', () => {
      const match = matchRoute('/watch/abc123');
      expect(match).toBeTruthy();
      expect(match?.params.ticket).toBe('abc123');
    });

    it('should match QR login with token', () => {
      const match = matchRoute('/qr-login/my-token-123');
      expect(match).toBeTruthy();
      expect(match?.params.token).toBe('my-token-123');
    });

    it('should match YouTube routes', () => {
      expect(matchRoute('/youtube')).toBeTruthy();
      expect(matchRoute('/youtube/recent')).toBeTruthy();
      expect(matchRoute('/youtube/playlists')).toBeTruthy();
    });

    it('should match YouTube channel with ID', () => {
      const match = matchRoute('/youtube/channel/UCxxxx123');
      expect(match).toBeTruthy();
      expect(match?.params.channel_id).toBe('UCxxxx123');
    });

    it('should match YouTube playlist with ID', () => {
      const match = matchRoute('/youtube/playlist/PLabc123');
      expect(match).toBeTruthy();
      expect(match?.params.playlist_id).toBe('PLabc123');
    });

    it('should match YouTube watch with video ID', () => {
      const match = matchRoute('/youtube/watch/abc123xyz');
      expect(match).toBeTruthy();
      expect(match?.params.video_id).toBe('abc123xyz');
    });

    it('should decode URL-encoded params', () => {
      const match = matchRoute('/twitch/channels/user%20name');
      expect(match?.params.login).toBe('user name');
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
