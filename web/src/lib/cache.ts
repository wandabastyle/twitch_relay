export interface CacheEntry<CacheType> {
  data: CacheType;
  timestamp: number;
}

const getFromCache = <CacheType>(key: string, maxAgeMs: number): CacheType | null => {
  if (globalThis.window === undefined) {
    return null;
  }

  try {
    const encoded = globalThis.window.sessionStorage.getItem(key);
    if (encoded === null || encoded === '') {
      return null;
    }

    const parsed = JSON.parse(encoded) as CacheEntry<CacheType>;
    const ageMs = Date.now() - parsed.timestamp;
    if (ageMs > maxAgeMs) {
      return null;
    }

    return parsed.data;
  } catch {
    return null;
  }
};

const setCache = <CacheType>(key: string, data: CacheType): void => {
  if (globalThis.window === undefined) {
    return;
  }

  try {
    const entry: CacheEntry<CacheType> = { data, timestamp: Date.now() };
    globalThis.window.sessionStorage.setItem(key, JSON.stringify(entry));
  } catch {
    // Ignore storage failures
  }
};

const clearCache = (key: string): void => {
  if (globalThis.window === undefined) {
    return;
  }

  try {
    globalThis.window.sessionStorage.removeItem(key);
  } catch {
    // Ignore storage failures
  }
};

export { clearCache, getFromCache, setCache };
