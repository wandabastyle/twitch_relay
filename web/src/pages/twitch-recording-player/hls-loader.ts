import type { HlsStatic } from './types';

const HLS_SCRIPT_PATH = '/static/hls.js';
const RESUME_WAIT_MS = 100;

const hasHlsLoaded = (): boolean => {
  const hlsValue: unknown = Reflect.get(globalThis, 'Hls');
  if (typeof hlsValue !== 'function') {
    return false;
  }
  return (
    'isSupported' in hlsValue &&
    'Events' in hlsValue &&
    typeof hlsValue.isSupported === 'function' &&
    typeof hlsValue.Events === 'object' &&
    hlsValue.Events !== null
  );
};

const delay = async (timeoutMs: number): Promise<void> => {
  await new Promise<void>((resolve) => {
    globalThis.setTimeout(() => {
      resolve();
    }, timeoutMs);
  });
};

const waitForHlsScript = async (): Promise<boolean> => {
  await delay(RESUME_WAIT_MS);
  return hasHlsLoaded();
};

const loadHlsScript = async (): Promise<boolean> => {
  const script = document.createElement('script');
  script.src = HLS_SCRIPT_PATH;
  script.async = true;
  const loaded = await new Promise<boolean>((resolve) => {
    script.addEventListener('load', () => {
      resolve(true);
    });
    script.addEventListener('error', () => {
      resolve(false);
    });
    document.head.append(script);
  });
  return loaded && hasHlsLoaded();
};

const hasExistingScript = (): boolean =>
  document.querySelector<HTMLScriptElement>(`script[src="${HLS_SCRIPT_PATH}"]`) !== null;

export const ensureHlsLoaded = (): Promise<boolean> => {
  if (typeof globalThis === 'undefined') {
    return Promise.resolve(false);
  }
  if (hasHlsLoaded()) {
    return Promise.resolve(true);
  }

  if (hasExistingScript()) {
    return waitForHlsScript();
  }

  return loadHlsScript();
};

const isValidHlsClass = (value: unknown): value is HlsStatic => {
  if (typeof value !== 'function') {
    return false;
  }
  if (!('isSupported' in value) || !('Events' in value)) {
    return false;
  }
  const typed = value as { isSupported: unknown; Events: unknown };
  if (typeof typed.isSupported !== 'function') {
    return false;
  }
  if (typeof typed.Events !== 'object' || typed.Events === null) {
    return false;
  }
  return true;
};

export const checkHlsSupport = (): { HlsClass: HlsStatic; supported: true } | { supported: false } => {
  const hlsValue: unknown = Reflect.get(globalThis, 'Hls');
  if (!isValidHlsClass(hlsValue)) {
    return { supported: false };
  }
  if (!hlsValue.isSupported()) {
    return { supported: false };
  }
  return { HlsClass: hlsValue, supported: true };
};
