import { describe, it, expect, beforeEach, vi } from "vitest";
import { getFromCache, setCache, clearCache, type CacheEntry } from "./cache";

describe("cache", () => {
  let mockStorage: Map<string, string>;
  let currentTime: number;

  beforeEach(() => {
    mockStorage = new Map();
    currentTime = 1000000;

    // Mock Date.now()
    vi.spyOn(Date, "now").mockImplementation(() => currentTime);

    // Mock sessionStorage
    Object.defineProperty(window, "sessionStorage", {
      value: {
        getItem: vi.fn((key: string) => mockStorage.get(key) ?? null),
        setItem: vi.fn((key: string, value: string) => {
          mockStorage.set(key, value);
        }),
        removeItem: vi.fn((key: string) => {
          mockStorage.delete(key);
        }),
      },
      writable: true,
    });
  });

  describe("getFromCache", () => {
    it("returns null when window is undefined (SSR)", () => {
      const originalWindow = globalThis.window;
      // @ts-expect-error - simulating SSR
      globalThis.window = undefined;

      const result = getFromCache("test-key", 60000);
      expect(result).toBeNull();

      globalThis.window = originalWindow;
    });

    it("returns null when key does not exist", () => {
      const result = getFromCache("non-existent-key", 60000);
      expect(result).toBeNull();
    });

    it("returns null when cached entry is expired", () => {
      const key = "expired-key";
      const entry: CacheEntry<string> = {
        timestamp: currentTime - 100001, // 100+ seconds ago
        data: "expired-data",
      };
      mockStorage.set(key, JSON.stringify(entry));

      const result = getFromCache(key, 100000); // 100s max age
      expect(result).toBeNull();
    });

    it("returns data when cached entry is still fresh", () => {
      const key = "fresh-key";
      const entry: CacheEntry<string> = {
        timestamp: currentTime - 50000, // 50 seconds ago
        data: "fresh-data",
      };
      mockStorage.set(key, JSON.stringify(entry));

      const result = getFromCache(key, 100000); // 100s max age
      expect(result).toBe("fresh-data");
    });

    it("returns null when JSON parsing fails (corrupted data)", () => {
      const key = "corrupted-key";
      mockStorage.set(key, "not-valid-json");

      const result = getFromCache(key, 60000);
      expect(result).toBeNull();
    });

    it("returns null when sessionStorage throws", () => {
      const key = "error-key";
      vi.mocked(window.sessionStorage.getItem).mockImplementation(() => {
        throw new Error("Storage error");
      });

      const result = getFromCache(key, 60000);
      expect(result).toBeNull();
    });
  });

  describe("setCache", () => {
    it("does nothing when window is undefined (SSR)", () => {
      const originalWindow = globalThis.window;
      // @ts-expect-error - simulating SSR
      globalThis.window = undefined;

      setCache("test-key", "test-data");
      // Should not throw

      globalThis.window = originalWindow;
    });

    it("stores data with current timestamp", () => {
      const key = "test-key";
      const data = { test: "value", number: 42 };

      setCache(key, data);

      const stored = mockStorage.get(key);
      expect(stored).toBeDefined();

      const parsed = JSON.parse(stored!) as CacheEntry<typeof data>;
      expect(parsed.timestamp).toBe(currentTime);
      expect(parsed.data).toEqual(data);
    });

    it("gracefully handles storage errors", () => {
      vi.mocked(window.sessionStorage.setItem).mockImplementation(() => {
        throw new Error("Quota exceeded");
      });

      // Should not throw
      setCache("test-key", "test-data");
    });
  });

  describe("clearCache", () => {
    it("does nothing when window is undefined (SSR)", () => {
      const originalWindow = globalThis.window;
      // @ts-expect-error - simulating SSR
      globalThis.window = undefined;

      clearCache("test-key");
      // Should not throw

      globalThis.window = originalWindow;
    });

    it("removes the key from storage", () => {
      const key = "test-key";
      mockStorage.set(key, JSON.stringify({ timestamp: currentTime, data: "test" }));

      clearCache(key);

      expect(mockStorage.has(key)).toBe(false);
    });

    it("gracefully handles errors when removing", () => {
      vi.mocked(window.sessionStorage.removeItem).mockImplementation(() => {
        throw new Error("Storage error");
      });

      // Should not throw
      clearCache("test-key");
    });
  });

  describe("integration", () => {
    it("full cache lifecycle works correctly", () => {
      const key = "lifecycle-key";
      const maxAge = 5000; // 5 seconds

      // Initially empty
      expect(getFromCache(key, maxAge)).toBeNull();

      // Set cache
      setCache(key, { value: "test" });
      expect(getFromCache(key, maxAge)).toEqual({ value: "test" });

      // Still valid after 3 seconds
      currentTime += 3000;
      expect(getFromCache(key, maxAge)).toEqual({ value: "test" });

      // Expired after 6 seconds total
      currentTime += 3000;
      expect(getFromCache(key, maxAge)).toBeNull();

      // Clear explicit
      setCache(key, { value: "new" });
      clearCache(key);
      expect(getFromCache(key, maxAge)).toBeNull();
    });
  });
});
