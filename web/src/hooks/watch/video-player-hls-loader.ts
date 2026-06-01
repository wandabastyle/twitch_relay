import {
  HLS_LOAD_ATTEMPTS,
  HLS_LOAD_INTERVAL_MS,
  HLS_POLL_INCREMENT,
} from './video-player-constants';

const waitForHls = async (): Promise<void> => {
  let attempts = 0;
  await new Promise<void>((resolve) => {
    const check = (): void => {
      if (
        typeof globalThis !== 'undefined' &&
        !('Hls' in globalThis) &&
        attempts < HLS_LOAD_ATTEMPTS
      ) {
        setTimeout(() => {
          attempts += HLS_POLL_INCREMENT;
          check();
        }, HLS_LOAD_INTERVAL_MS);
      } else {
        resolve();
      }
    };
    check();
  });
};

const loadScript = async (path: string): Promise<boolean> => {
  const script = document.createElement('script');
  script.src = path;
  script.async = true;

  const result = await new Promise<boolean>((resolve) => {
    script.addEventListener('load', () => {
      resolve(true);
    });
    script.addEventListener('error', () => {
      resolve(false);
    });
    document.head.append(script);
  });
  return result;
};

export const ensureHlsLoaded = async (path: string): Promise<boolean> => {
  if (typeof globalThis === 'undefined') {
    return false;
  }
  if ('Hls' in globalThis) {
    return true;
  }

  const existing = document.querySelector<HTMLScriptElement>(`script[src="${path}"]`);
  if (existing) {
    await waitForHls();
    return 'Hls' in globalThis;
  }

  const loaded = await loadScript(path);
  if (!loaded) {
    return false;
  }
  await waitForHls();
  return 'Hls' in globalThis;
};
