import { beforeEach, describe, expect, it, vi } from 'vitest';
import { clearCache, getFromCache, setCache, type CacheEntry } from './cache';

const INITIAL_TIME = 1_000_000;
const MAX_AGE_MS = 60_000;
const EXPIRED_TIME_OFFSET = 100_001;
const FRESH_TIME_OFFSET = 50_000;
const CACHE_CHECK_TIME_OFFSET = 3000;

// Mock storage interface to avoid unbound method warnings
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
    removeItem: (key: string): void => {
      storage.delete(key);
    },
    setItem: (key: string, value: string): void => {
      storage.set(key, value);
    },
    get length() {
      return storage.size;
    },
  };
  return Object.assign(mockStorage, { sessionStorage: mockStorage }) as Storage & {
    sessionStorage: Storage;
  };
};

describe('cache', () => {
  let currentTime: number;

  beforeEach(() => {
    currentTime = INITIAL_TIME;

    // Mock Date.now()
    vi.spyOn(Date, 'now').mockImplementation(() => currentTime);

    // Mock sessionStorage with bound methods
    Object.defineProperty(globalThis, 'window', {
      value: createMockStorage(),
      writable: true,
    });
  });

  describe('getFromCache', () => {
    it('returns null when window is undefined (SSR)', () => {
      const originalWindow = globalThis.window;
      // @ts-expect-error - simulating SSR
      globalThis.window = undefined;

      const result = getFromCache('test-key', MAX_AGE_MS);
      expect(result).toBeNull();

      globalThis.window = originalWindow;
    });

    it('returns null when key does not exist', () => {
      const result = getFromCache('non-existent-key', MAX_AGE_MS);
      expect(result).toBeNull();
    });

    it('returns null when cached entry is expired', () => {
      const key = 'expired-key';
      const entry: CacheEntry<string> = {
        data: 'expired-data',
        timestamp: currentTime - EXPIRED_TIME_OFFSET,
      };
      globalThis.window.sessionStorage.setItem(key, JSON.stringify(entry));

      const result = getFromCache(key, MAX_AGE_MS);
      expect(result).toBeNull();
    });

    it('returns data when cached entry is still fresh', () => {
      const key = 'fresh-key';
      const entry: CacheEntry<string> = {
        data: 'fresh-data',
        timestamp: currentTime - FRESH_TIME_OFFSET,
      };
      globalThis.window.sessionStorage.setItem(key, JSON.stringify(entry));

      const result = getFromCache(key, MAX_AGE_MS);
      expect(result).toBe('fresh-data');
    });

    it('returns null when JSON parsing fails (corrupted data)', () => {
      const key = 'corrupted-key';
      globalThis.window.sessionStorage.setItem(key, 'not-valid-json');

      const result = getFromCache(key, MAX_AGE_MS);
      expect(result).toBeNull();
    });

    it('returns null when sessionStorage throws', () => {
      const key = 'error-key';
      // Store reference to original before mocking
      const originalGetItem = globalThis.window.sessionStorage.getItem.bind(
        globalThis.window.sessionStorage,
      );
      globalThis.window.sessionStorage.getItem = (): string | null => {
        throw new Error('Storage error');
      };

      const result = getFromCache(key, MAX_AGE_MS);
      expect(result).toBeNull();

      // Restore original
      globalThis.window.sessionStorage.getItem = originalGetItem;
    });
  });

  describe('setCache', () => {
    it('does nothing when window is undefined (SSR)', () => {
      const originalWindow = globalThis.window;
      // @ts-expect-error - simulating SSR
      globalThis.window = undefined;

      setCache('test-key', 'test-data');
      // Should not throw

      globalThis.window = originalWindow;
    });

    it('stores data with current timestamp', () => {
      const key = 'test-key';
      const data = { number: 42, test: 'value' };

      setCache(key, data);

      const stored = globalThis.window.sessionStorage.getItem(key);
      expect(stored).toBeDefined();

      const parsed = JSON.parse(stored!) as CacheEntry<typeof data>;
      expect(parsed.timestamp).toBe(currentTime);
      expect(parsed.data).toEqual(data);
    });

    it('gracefully handles storage errors', () => {
      const originalSetItem = globalThis.window.sessionStorage.setItem.bind(
        globalThis.window.sessionStorage,
      );
      globalThis.window.sessionStorage.setItem = (_key: string, _value: string): void => {
        throw new Error('Quota exceeded');
      };

      // Should not throw
      setCache('test-key', 'test-data');

      // Restore original
      globalThis.window.sessionStorage.setItem = originalSetItem;
    });
  });

  describe('clearCache', () => {
    it('does nothing when window is undefined (SSR)', () => {
      const originalWindow = globalThis.window;
      // @ts-expect-error - simulating SSR
      globalThis.window = undefined;

      clearCache('test-key');
      // Should not throw

      globalThis.window = originalWindow;
    });

    it('removes the key from storage', () => {
      const key = 'test-key';
      const entry: CacheEntry<string> = {
        data: 'test',
        timestamp: currentTime,
      };
      globalThis.window.sessionStorage.setItem(key, JSON.stringify(entry));

      clearCache(key);

      expect(globalThis.window.sessionStorage.getItem(key)).toBeNull();
    });

    it('gracefully handles errors when removing', () => {
      const originalRemoveItem = globalThis.window.sessionStorage.removeItem.bind(
        globalThis.window.sessionStorage,
      );
      globalThis.window.sessionStorage.removeItem = (_key: string): void => {
        throw new Error('Storage error');
      };

      // Should not throw
      clearCache('test-key');

      // Restore original
      globalThis.window.sessionStorage.removeItem = originalRemoveItem;
    });
  });

  describe('integration', () => {
    const MAX_AGE_SECONDS = 5000;

    it('full cache lifecycle works correctly', () => {
      const key = 'lifecycle-key';

      // Initially empty
      expect(getFromCache(key, MAX_AGE_SECONDS)).toBeNull();

      // Set cache
      setCache(key, { value: 'test' });
      expect(getFromCache(key, MAX_AGE_SECONDS)).toEqual({ value: 'test' });

      // Still valid after 3 seconds
      currentTime += CACHE_CHECK_TIME_OFFSET;
      expect(getFromCache(key, MAX_AGE_SECONDS)).toEqual({ value: 'test' });

      // Expired after 6 seconds total
      currentTime += CACHE_CHECK_TIME_OFFSET;
      expect(getFromCache(key, MAX_AGE_SECONDS)).toBeNull();

      // Clear explicit
      setCache(key, { value: 'new' });
      clearCache(key);
      expect(getFromCache(key, MAX_AGE_SECONDS)).toBeNull();
    });
  });
});
