export interface CacheEntry {
  data: unknown;
  timestamp: number;
}

const isBrowser = (): boolean => 'window' in globalThis;

const isObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null;

const hasProperty = <Obj extends object, Key extends string>(
  obj: Obj,
  prop: Key,
): obj is Obj & Record<Key, unknown> => prop in obj;

const isCacheEntry = (value: unknown): value is CacheEntry => {
  if (!isObject(value)) {
    return false;
  }
  if (!hasProperty(value, 'data')) {
    return false;
  }
  if (!hasProperty(value, 'timestamp')) {
    return false;
  }
  return typeof value.timestamp === 'number';
};

const parseCacheEntry = (encoded: string): CacheEntry | null => {
  try {
    const parsed: unknown = JSON.parse(encoded);
    if (!isCacheEntry(parsed)) {
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
};

const isCacheExpired = (entry: Readonly<CacheEntry>, maxAgeMs: number): boolean => {
  const ageMs = Date.now() - entry.timestamp;
  return ageMs > maxAgeMs;
};

const clearCache = (key: string): void => {
  if (!isBrowser()) {
    return;
  }

  try {
    globalThis.window.sessionStorage.removeItem(key);
  } catch {
    // Ignore storage failures
  }
};

const readStorageItem = (key: string): string | null => {
  try {
    return globalThis.window.sessionStorage.getItem(key);
  } catch {
    return null;
  }
};

const readCacheValue = (key: string, maxAgeMs: number): unknown => {
  if (!isBrowser()) {
    return null;
  }

  const encoded = readStorageItem(key);
  if (encoded === null || encoded === '') {
    return null;
  }

  const parsed = parseCacheEntry(encoded);
  if (parsed === null || isCacheExpired(parsed, maxAgeMs)) {
    return null;
  }
  return parsed.data;
};

const getFromCache = (key: string, maxAgeMs: number): unknown => readCacheValue(key, maxAgeMs);

const setCache = (key: string, data: unknown): void => {
  if (!isBrowser()) {
    return;
  }

  try {
    const entry: CacheEntry = {
      data,
      timestamp: Date.now(),
    };
    globalThis.window.sessionStorage.setItem(key, JSON.stringify(entry));
  } catch {
    // Ignore storage failures
  }
};

export { clearCache, getFromCache, setCache };
