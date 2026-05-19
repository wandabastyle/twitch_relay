import { describe, it, expect, beforeEach, vi } from 'vitest';
import { getFromCache, setCache, clearCache, type CacheEntry } from './cache';

describe('cache', () => {
  let currentTime: number;

  // Mock storage interface to avoid unbound method warnings
  function createMockStorage(): Storage {
    const storage = new Map<string, string>();
    return {
      get length() {
        return storage.size;
      },
      key(index: number) {
        const keys = Array.from(storage.keys());
        return keys[index] ?? null;
      },
      getItem: (key: string): string | null => storage.get(key) ?? null,
      setItem: (key: string, value: string): void => {
        storage.set(key, value);
      },
      removeItem: (key: string): void => {
        storage.delete(key);
      },
      clear: (): void => {
        storage.clear();
      },
    } as Storage;
  }

  beforeEach(() => {
    currentTime = 1000000;

    // Mock Date.now()
    vi.spyOn(Date, 'now').mockImplementation(() => currentTime);

    // Mock sessionStorage with bound methods
    Object.defineProperty(window, 'sessionStorage', {
      value: createMockStorage(),
      writable: true,
    });
  });

  describe('getFromCache', () => {
    it('returns null when window is undefined (SSR)', () => {
      const originalWindow = globalThis.window;
      // @ts-expect-error - simulating SSR
      globalThis.window = undefined;

      const result = getFromCache('test-key', 60000);
      expect(result).toBeNull();

      globalThis.window = originalWindow;
    });

    it('returns null when key does not exist', () => {
      const result = getFromCache('non-existent-key', 60000);
      expect(result).toBeNull();
    });

    it('returns null when cached entry is expired', () => {
      const key = 'expired-key';
      const entry: CacheEntry<string> = {
        timestamp: currentTime - 100001, // 100+ seconds ago
        data: 'expired-data',
      };
      window.sessionStorage.setItem(key, JSON.stringify(entry));

      const result = getFromCache(key, 100000); // 100s max age
      expect(result).toBeNull();
    });

    it('returns data when cached entry is still fresh', () => {
      const key = 'fresh-key';
      const entry: CacheEntry<string> = {
        timestamp: currentTime - 50000, // 50 seconds ago
        data: 'fresh-data',
      };
      window.sessionStorage.setItem(key, JSON.stringify(entry));

      const result = getFromCache(key, 100000); // 100s max age
      expect(result).toBe('fresh-data');
    });

    it('returns null when JSON parsing fails (corrupted data)', () => {
      const key = 'corrupted-key';
      window.sessionStorage.setItem(key, 'not-valid-json');

      const result = getFromCache(key, 60000);
      expect(result).toBeNull();
    });

    it('returns null when sessionStorage throws', () => {
      const key = 'error-key';
      // Store reference to original before mocking
      const originalGetItem = window.sessionStorage.getItem.bind(window.sessionStorage);
      window.sessionStorage.getItem = (): string | null => {
        throw new Error('Storage error');
      };

      const result = getFromCache(key, 60000);
      expect(result).toBeNull();

      // Restore original
      window.sessionStorage.getItem = originalGetItem;
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
      const data = { test: 'value', number: 42 };

      setCache(key, data);

      const stored = window.sessionStorage.getItem(key);
      expect(stored).toBeDefined();

      const parsed = JSON.parse(stored!) as CacheEntry<typeof data>;
      expect(parsed.timestamp).toBe(currentTime);
      expect(parsed.data).toEqual(data);
    });

    it('gracefully handles storage errors', () => {
      const originalSetItem = window.sessionStorage.setItem.bind(window.sessionStorage);
      window.sessionStorage.setItem = (_key: string, _value: string): void => {
        throw new Error('Quota exceeded');
      };

      // Should not throw
      setCache('test-key', 'test-data');

      // Restore original
      window.sessionStorage.setItem = originalSetItem;
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
      window.sessionStorage.setItem(key, JSON.stringify({ timestamp: currentTime, data: 'test' }));

      clearCache(key);

      expect(window.sessionStorage.getItem(key)).toBeNull();
    });

    it('gracefully handles errors when removing', () => {
      const originalRemoveItem = window.sessionStorage.removeItem.bind(window.sessionStorage);
      window.sessionStorage.removeItem = (_key: string): void => {
        throw new Error('Storage error');
      };

      // Should not throw
      clearCache('test-key');

      // Restore original
      window.sessionStorage.removeItem = originalRemoveItem;
    });
  });

  describe('integration', () => {
    it('full cache lifecycle works correctly', () => {
      const key = 'lifecycle-key';
      const maxAge = 5000; // 5 seconds

      // Initially empty
      expect(getFromCache(key, maxAge)).toBeNull();

      // Set cache
      setCache(key, { value: 'test' });
      expect(getFromCache(key, maxAge)).toEqual({ value: 'test' });

      // Still valid after 3 seconds
      currentTime += 3000;
      expect(getFromCache(key, maxAge)).toEqual({ value: 'test' });

      // Expired after 6 seconds total
      currentTime += 3000;
      expect(getFromCache(key, maxAge)).toBeNull();

      // Clear explicit
      setCache(key, { value: 'new' });
      clearCache(key);
      expect(getFromCache(key, maxAge)).toBeNull();
    });
  });
});
