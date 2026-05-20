import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getCachedLiveStatus, getLiveStatus } from './channels.js';

// Test constants
const CACHE_KEY = 'twitchRelay.liveStatus';
const TEST_TIMESTAMP = 1_000_000;

// Cache timing
// Duration of 30 seconds
const FRESH_OFFSET = 30_000;
// Duration of 1 minute cache
const MAX_AGE_MS = 60_000;
const EXPIRE_MARGIN = 1000;

// Sample values
const VIEWER_COUNT_100 = 100;
const VIEWER_COUNT_1000 = 1000;
const VIEWER_COUNT_12345 = 12_345;
const VIEWER_COUNT_50000 = 50_000;
const EXPECTED_TWO_CHANNELS = 2;
const MIN_CHANNEL_COUNT = 0;

// HTTP status codes
const STATUS_OK = 200;
const STATUS_SERVER_ERROR = 500;

/**
 * Create a mock Response with JSON body
 */
const createMockResponse = (body: unknown, status = STATUS_OK): Response =>
  Response.json(body, { status });

/**
 * Create a cache entry for sessionStorage
 */
const createCacheEntry = (data: unknown, timestamp: number): string =>
  JSON.stringify({ data, timestamp });

describe('api-client/channels getCachedLiveStatus', () => {
  beforeEach(() => {
    // Clear sessionStorage before each test
    const mockStorage = new Map<string, string>();

    Object.defineProperty(globalThis, 'window', {
      value: {
        sessionStorage: {
          getItem: (key: string): string | null => mockStorage.get(key) ?? null,
          removeItem: (key: string): void => {
            mockStorage.delete(key);
          },
          setItem: (key: string, value: string): void => {
            mockStorage.set(key, value);
          },
        },
      },
      writable: true,
    });
    vi.spyOn(Date, 'now').mockReturnValue(TEST_TIMESTAMP);
  });

  it('returns empty object when no cached data exists', () => {
    const result = getCachedLiveStatus();

    expect(result).toEqual({});
  });

  it('returns empty object when cache entry is invalid', () => {
    globalThis.window.sessionStorage.setItem(CACHE_KEY, 'invalid-json');
    const result = getCachedLiveStatus();

    expect(result).toEqual({});
  });

  it('returns empty object when cache is expired', () => {
    const expiredData = {
      channels: {
        testuser: { live: true, viewer_count: 1234 },
      },
    };
    const expiredTimestamp = TEST_TIMESTAMP - MAX_AGE_MS - EXPIRE_MARGIN;

    globalThis.window.sessionStorage.setItem(
      CACHE_KEY,
      createCacheEntry(expiredData, expiredTimestamp),
    );
    const result = getCachedLiveStatus();

    expect(result).toEqual({});
  });

  it('returns cached live status from fresh cache entry', () => {
    const cachedData = {
      channels: {
        liveuser: { live: true, viewer_count: 1234 },
        offlineuser: { live: false },
      },
    };
    const freshTimestamp = TEST_TIMESTAMP - FRESH_OFFSET;

    globalThis.window.sessionStorage.setItem(
      CACHE_KEY,
      createCacheEntry(cachedData, freshTimestamp),
    );
    const result = getCachedLiveStatus();

    expect(result).toEqual({
      liveuser: { live: true, viewer_count: 1234 },
      offlineuser: { live: false },
    });
  });

  it('returns empty object when cache data has no channels property', () => {
    const invalidData = { notChannels: {} };

    globalThis.window.sessionStorage.setItem(
      CACHE_KEY,
      createCacheEntry(invalidData, TEST_TIMESTAMP),
    );
    const result = getCachedLiveStatus();

    expect(result).toEqual({});
  });
});

describe('api-client/channels getLiveStatus', () => {
  beforeEach(() => {
    const mockStorage = new Map<string, string>();

    Object.defineProperty(globalThis, 'window', {
      value: {
        sessionStorage: {
          getItem: (key: string): string | null => mockStorage.get(key) ?? null,
          removeItem: (key: string): void => {
            mockStorage.delete(key);
          },
          setItem: (key: string, value: string): void => {
            mockStorage.set(key, value);
          },
        },
      },
      writable: true,
    });
    vi.spyOn(Date, 'now').mockReturnValue(TEST_TIMESTAMP);
    vi.restoreAllMocks();
  });

  it('fetches live status from API when no cache exists', async () => {
    const apiResponse = {
      channels: {
        streamer1: { live: true, title: 'Test Stream', viewer_count: 5000 },
        streamer2: { live: false },
      },
    };
    const mockFetch = vi.fn().mockResolvedValue(createMockResponse(apiResponse));

    globalThis.fetch = mockFetch;
    const result = await getLiveStatus();

    expect(result).toEqual(apiResponse);
    expect(mockFetch).toHaveBeenCalledWith('/api/live-status', { credentials: 'same-origin' });
  });

  it('returns cached data and refreshes in background', async () => {
    // Setup cached data
    const cachedData = {
      channels: {
        oldstreamer: { live: false },
      },
    };

    globalThis.window.sessionStorage.setItem(
      CACHE_KEY,
      createCacheEntry(cachedData, TEST_TIMESTAMP - FRESH_OFFSET),
    );

    // Setup API response for refresh
    const apiResponse = {
      channels: {
        newstreamer: { live: true, viewer_count: VIEWER_COUNT_1000 },
      },
    };
    const mockFetch = vi.fn().mockResolvedValue(createMockResponse(apiResponse));

    globalThis.fetch = mockFetch;
    const result = await getLiveStatus();

    // Should have triggered a background refresh
    expect(mockFetch).toHaveBeenCalled();
    // Result should include live status data
    expect(result.channels).toBeDefined();
    expect(Object.keys(result.channels).length).toBeGreaterThan(MIN_CHANNEL_COUNT);
  });

  it('correctly parses offline channels with missing optional fields', async () => {
    // Channel without viewer_count, game, title, profile_url, display_name
    const apiResponse = {
      channels: {
        offlineuser: { live: false },
      },
    };
    const mockFetch = vi.fn().mockResolvedValue(createMockResponse(apiResponse));

    globalThis.fetch = mockFetch;
    const result = await getLiveStatus();

    expect(result).toEqual(apiResponse);
    expect(result.channels.offlineuser.live).toBe(false);
    expect(result.channels.offlineuser.viewer_count).toBeUndefined();
  });

  it('correctly parses live channels with all optional fields', async () => {
    const apiResponse = {
      channels: {
        liveuser: {
          display_name: 'LiveUser',
          game: 'Just Chatting',
          live: true,
          profile_url: 'https://example.com/profile.png',
          title: 'Hanging out!',
          viewer_count: VIEWER_COUNT_50000,
        },
      },
    };
    const mockFetch = vi.fn().mockResolvedValue(createMockResponse(apiResponse));

    globalThis.fetch = mockFetch;
    const result = await getLiveStatus();

    expect(result).toEqual(apiResponse);
    expect(result.channels.liveuser.live).toBe(true);
    expect(result.channels.liveuser.viewer_count).toBe(VIEWER_COUNT_50000);
    expect(result.channels.liveuser.game).toBe('Just Chatting');
  });

  it('filters out invalid channel status entries', async () => {
    // Backend might return malformed data - ensure it is filtered
    const apiResponse = {
      channels: {
        // Missing live field
        anotherinvalid: { viewer_count: 50 },
        // Wrong field name
        invaliduser: { is_live: true },
        validuser: { live: true, viewer_count: VIEWER_COUNT_100 },
      },
    };
    const mockFetch = vi.fn().mockResolvedValue(createMockResponse(apiResponse));

    globalThis.fetch = mockFetch;
    const result = await getLiveStatus();

    // Only validuser should be present
    expect(result.channels.validuser).toBeDefined();
    expect(result.channels.invaliduser).toBeUndefined();
    expect(result.channels.anotherinvalid).toBeUndefined();
  });

  it('handles API error gracefully', async () => {
    const mockFetch = vi
      .fn()
      .mockResolvedValue(Response.json({ error: 'Server error' }, { status: STATUS_SERVER_ERROR }));

    globalThis.fetch = mockFetch;
    await expect(getLiveStatus()).rejects.toThrow();
  });

  it('throws when response payload is not an object', async () => {
    const mockFetch = vi.fn().mockResolvedValue(createMockResponse('not-an-object'));

    globalThis.fetch = mockFetch;
    await expect(getLiveStatus()).rejects.toThrow('live status payload is invalid');
  });

  it('throws when response channels property is not an object', async () => {
    const mockFetch = vi.fn().mockResolvedValue(createMockResponse({ channels: 'not-an-object' }));

    globalThis.fetch = mockFetch;
    await expect(getLiveStatus()).rejects.toThrow('live status payload is invalid');
  });
});

describe('api-client/channels live status parsing', () => {
  beforeEach(() => {
    const mockStorage = new Map<string, string>();

    Object.defineProperty(globalThis, 'window', {
      value: {
        sessionStorage: {
          getItem: (key: string): string | null => mockStorage.get(key) ?? null,
          removeItem: (key: string): void => {
            mockStorage.delete(key);
          },
          setItem: (key: string, value: string): void => {
            mockStorage.set(key, value);
          },
        },
      },
      writable: true,
    });
    vi.spyOn(Date, 'now').mockReturnValue(TEST_TIMESTAMP);
    vi.restoreAllMocks();
  });

  it('matches Rust backend serialization format (live field)', async () => {
    // This test ensures frontend accepts backend format
    // Rust backend sends: { "live": true, "viewer_count": 123 }
    // Not: { "is_live": true, ... }
    const backendFormat = {
      channels: {
        offlineuser: {
          live: false,
          // Optional fields omitted when offline
        },
        rustuser: {
          display_name: 'RustUser',
          game: 'Rust Programming',
          live: true,
          profile_url: 'https://example.com/rust.png',
          title: 'Coding Stream',
          viewer_count: VIEWER_COUNT_12345,
        },
      },
    };
    const mockFetch = vi.fn().mockResolvedValue(createMockResponse(backendFormat));

    globalThis.fetch = mockFetch;
    const result = await getLiveStatus();

    // All channels should be parsed correctly
    expect(Object.keys(result.channels)).toHaveLength(EXPECTED_TWO_CHANNELS);
    expect(result.channels.rustuser).toEqual(backendFormat.channels.rustuser);
    expect(result.channels.offlineuser).toEqual(backendFormat.channels.offlineuser);
  });
});
