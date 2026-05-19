export interface CacheEntry<T> {
  timestamp: number;
  data: T;
}

export function getFromCache<T>(key: string, maxAgeMs: number): T | null {
  if (typeof window === "undefined") return null;

  try {
    const encoded = window.sessionStorage.getItem(key);
    if (!encoded) return null;

    const parsed = JSON.parse(encoded) as CacheEntry<T>;
    const ageMs = Date.now() - parsed.timestamp;
    if (ageMs > maxAgeMs) return null;

    return parsed.data;
  } catch {
    return null;
  }
}

export function setCache<T>(key: string, data: T): void {
  if (typeof window === "undefined") return;

  try {
    const entry: CacheEntry<T> = { timestamp: Date.now(), data };
    window.sessionStorage.setItem(key, JSON.stringify(entry));
  } catch {
    // Ignore storage failures
  }
}

export function clearCache(key: string): void {
  if (typeof window === "undefined") return;

  try {
    window.sessionStorage.removeItem(key);
  } catch {
    // Ignore storage failures
  }
}
