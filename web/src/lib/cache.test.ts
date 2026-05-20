import { beforeEach, describe, expect, it, vi } from 'vitest';

import { clearCache, getFromCache, setCache, type CacheEntry } from './cache';

const INITIAL_TIME = 1_000_000;
const MAX_AGE_MS = 60_000;
const EXPIRED_TIME_OFFSET = 100_001;
const FRESH_TIME_OFFSET = 50_000;
const CACHE_CHECK_TIME_OFFSET = 3000;
const LIFECYCLE_MAX_AGE_MS = 5000;

const createMockStorage = (): Storage & { sessionStorage: Storage } => {
  const storage = new Map<string, string>();
  const mockStorage: Storage = {
    clear: (): void => {
      storage.clear();
    },
    getItem: (key: string): string | null => storage.get(key) ?? null,
    key(index: number) {
      const keys = [...storage.keys()];
      return keys[index] ?? null;
    },
    get length() {
      return storage.size;
    },
    removeItem: (key: string): void => {
      storage.delete(key);
    },
    setItem: (key: string, value: string): void => {
      storage.set(key, value);
    },
  };
  return Object.assign(mockStorage, { sessionStorage: mockStorage }) as Storage & {
    sessionStorage: Storage;
  };
};

let currentTime = INITIAL_TIME;

const installBrowserMocks = (): void => {
  currentTime = INITIAL_TIME;
  vi.spyOn(Date, 'now').mockImplementation(() => currentTime);
  Object.defineProperty(globalThis, 'window', {
    value: createMockStorage(),
    writable: true,
  });
};

const withSsrWindow = (callback: () => void): void => {
  const originalWindow = globalThis.window;
  // @ts-expect-error testing SSR
  globalThis.window = undefined;
  callback();
  globalThis.window = originalWindow;
};

const parseStoredEntry = (key: string): { readonly timestamp: number; readonly data: unknown } => {
  const stored = globalThis.window.sessionStorage.getItem(key);
  if (stored === null) {
    throw new Error('Stored cache entry should exist');
  }
  const parsed: unknown = JSON.parse(stored);
  if (
    typeof parsed !== 'object' ||
    parsed === null ||
    !('timestamp' in parsed) ||
    !('data' in parsed)
  ) {
    throw new Error('Invalid parsed cache entry');
  }
  if (typeof parsed.timestamp !== 'number') {
    throw new TypeError('Invalid timestamp in parsed cache entry');
  }
  return { data: parsed.data, timestamp: parsed.timestamp };
};

const expectLifecycleState = (key: string, maxAgeMs: number, expected: unknown): void => {
  expect(getFromCache(key, maxAgeMs)).toEqual(expected);
};

beforeEach(() => {
  installBrowserMocks();
});

describe('cache getFromCache', () => {
  it('returns null when window is undefined (SSR)', () => {
    withSsrWindow(() => {
      expect(getFromCache('test-key', MAX_AGE_MS)).toBeNull();
    });
  });

  it('returns null when key does not exist', () => {
    expect(getFromCache('non-existent-key', MAX_AGE_MS)).toBeNull();
  });

  it('returns null when cached entry is expired', () => {
    const key = 'expired-key';
    const entry: CacheEntry = {
      data: 'expired-data',
      timestamp: currentTime - EXPIRED_TIME_OFFSET,
    };
    globalThis.window.sessionStorage.setItem(key, JSON.stringify(entry));
    expect(getFromCache(key, MAX_AGE_MS)).toBeNull();
  });

  it('returns data when cache entry is fresh', () => {
    const key = 'fresh-key';
    const entry: CacheEntry = {
      data: 'fresh-data',
      timestamp: currentTime - FRESH_TIME_OFFSET,
    };
    globalThis.window.sessionStorage.setItem(key, JSON.stringify(entry));
    expect(getFromCache(key, MAX_AGE_MS)).toBe('fresh-data');
  });
});

describe('cache setCache', () => {
  it('does nothing when window is undefined (SSR)', () => {
    withSsrWindow(() => {
      setCache('test-key', 'test-data');
    });
  });

  it('stores data with current timestamp', () => {
    const key = 'test-key';
    const data = { number: 42, test: 'value' };
    setCache(key, data);
    const parsed = parseStoredEntry(key);
    expect(parsed.timestamp).toBe(currentTime);
    expect(parsed.data).toEqual(data);
  });
});

describe('cache clearCache', () => {
  it('does nothing when window is undefined (SSR)', () => {
    withSsrWindow(() => {
      clearCache('test-key');
    });
  });

  it('removes key from storage', () => {
    const key = 'test-key';
    setCache(key, { value: 'test' });
    clearCache(key);
    expect(globalThis.window.sessionStorage.getItem(key)).toBeNull();
  });
});

describe('cache integration', () => {
  const setupLifecycle = (key: string): void => {
    setCache(key, { value: 'test' });
    expectLifecycleState(key, LIFECYCLE_MAX_AGE_MS, { value: 'test' });
  };

  const expireLifecycle = (key: string): void => {
    currentTime += CACHE_CHECK_TIME_OFFSET;
    expectLifecycleState(key, LIFECYCLE_MAX_AGE_MS, { value: 'test' });
    currentTime += CACHE_CHECK_TIME_OFFSET;
    expect(getFromCache(key, LIFECYCLE_MAX_AGE_MS)).toBeNull();
  };

  it('supports full cache lifecycle', () => {
    const key = 'lifecycle-key';
    const maxAgeMs = LIFECYCLE_MAX_AGE_MS;

    expect(getFromCache(key, maxAgeMs)).toBeNull();
    setupLifecycle(key);
    expireLifecycle(key);

    setCache(key, { value: 'new' });
    clearCache(key);
    expect(getFromCache(key, maxAgeMs)).toBeNull();
  });
});
