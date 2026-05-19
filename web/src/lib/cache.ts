export interface CacheEntry<CacheType> {
  data: CacheType;
  timestamp: number;
}

const isBrowser = (): boolean => typeof globalThis.window !== 'undefined';

const isObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null;

const hasProperty = <Obj extends object, Key extends string>(
  obj: Obj,
  prop: Key,
): obj is Obj & Record<Key, unknown> => prop in obj;

const isCacheEntry = <CacheType>(value: unknown): value is CacheEntry<CacheType> => {
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

const parseCacheEntry = <CacheType>(encoded: string): CacheEntry<CacheType> | null => {
  try {
    const parsed: unknown = JSON.parse(encoded);
    if (!isCacheEntry<CacheType>(parsed)) {
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
};

const isCacheExpired = <CacheType>(
  entry: Readonly<CacheEntry<CacheType>>,
  maxAgeMs: number,
): boolean => {
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

const getFromCache = <CacheType>(key: string, maxAgeMs: number): CacheType | null => {
  if (!isBrowser()) {
    return null;
  }

  try {
    const encoded = globalThis.window.sessionStorage.getItem(key);
    if (encoded === null || encoded === '') {
      return null;
    }

    const parsed = parseCacheEntry<CacheType>(encoded);
    if (parsed === null || isCacheExpired(parsed, maxAgeMs)) {
      return null;
    }

    return parsed.data;
  } catch {
    return null;
  }
};

const setCache = <CacheType>(key: string, data: CacheType): void => {
  if (!isBrowser()) {
    return;
  }

  try {
    const entry: CacheEntry<CacheType> = {
      data,
      timestamp: Date.now(),
    };
    globalThis.window.sessionStorage.setItem(key, JSON.stringify(entry));
  } catch {
    // Ignore storage failures
  }
};

export { clearCache, getFromCache, setCache };
